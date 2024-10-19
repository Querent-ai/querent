use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{Stream, StreamExt};
use image::guess_format;
use proto::semantics::IngestedTokens;
use rayon::prelude::*;
use std::{
	io::{Cursor, Read},
	path::PathBuf,
	pin::Pin,
	sync::Arc,
};
use tokio::{io::AsyncReadExt, task::spawn_blocking};
use tracing::{error, info};
use xml::reader::{EventReader, XmlEvent};
use zip::ZipArchive;

use crate::{
	image::image::ImageIngestor, process_ingested_tokens_stream,
	processors::text_processing::TextCleanupProcessor, AsyncProcessor, BaseIngestor,
	IngestorResult,
};

// Define the OdpIngestor
pub struct OdpIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl OdpIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}
}

#[async_trait]
impl BaseIngestor for OdpIngestor {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
		self.processors = processors;
	}

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
	{
		let stream = stream! {
			// Collect all the bytes into a single buffer
			let mut buffer = Vec::new();
			let mut file = String::new();
			let mut doc_source = String::new();
			let mut source_id = String::new();

			for collected_bytes in all_collected_bytes {
				if collected_bytes.data.is_none() || collected_bytes.file.is_none() {
					continue;
				}
				if file.is_empty() {
					file = collected_bytes.file.as_ref().unwrap().to_string_lossy().to_string();
				}
				if doc_source.is_empty() {
					doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
				}
				if let Some(mut data) = collected_bytes.data {
					let mut buf = Vec::new();
					data.read_to_end(&mut buf).await.unwrap();
					buffer.extend_from_slice(&buf);
				}
				source_id = collected_bytes.source_id.clone();
			}

			let cursor = Cursor::new(buffer);
			let mut archive = match ZipArchive::new(cursor) {
				Ok(archive) => archive,
				Err(e) => {
					error!("Failed to read zip archive: {:?}", e);
					return;
				}
			};

			let mut xml_data = String::new();
			let mut slide_images = Vec::new();

			// Extract content.xml and images
			for i in 0..archive.len() {
				let mut file = match archive.by_index(i) {
					Ok(file) => file,
					Err(e) => {
						error!("Failed to access file in archive: {:?}", e);
						return;
					}
				};

				// Read the XML content
				if file.name() == "content.xml" {
					if let Err(e) = file.read_to_string(&mut xml_data) {
						error!("Failed to read XML data: {:?}", e);
						return;
					}
				}

				// Extract images
				if file.name().starts_with("Pictures/") {
					let mut img_data = Vec::new();
					if let Err(e) = file.read_to_end(&mut img_data) {
						error!("Failed to read image data: {:?}", e);
						return;
					}
					slide_images.push((file.name().to_string(), img_data));
				}
			}

			if xml_data.is_empty() {
				error!("No content.xml found in the archive or the file is empty");
				return;
			}

			let reader = EventReader::from_str(&xml_data);
			let mut text = String::new();
			let mut to_read = false;

			for e in reader {
				match e {
					Ok(XmlEvent::StartElement { name, .. }) => {
						if name.local_name == "p" || name.local_name == "span" {
							to_read = true;
						}
					}
					Ok(XmlEvent::Characters(data)) => {
						if to_read {
							text.push_str(&data);
						}
					}
					Ok(XmlEvent::EndElement { name }) => {
						if name.local_name == "p" {
							text.push_str("\n\n");
							to_read = false;
						}
					}
					Err(e) => {
						error!("Error reading XML: {:?}", e);
						return;
					}
					_ => {}
				}
			}

			if text.is_empty() && slide_images.is_empty() {
				info!("No content.xml found in the archive or the file is empty");
				return;
			}

			// Yield the text tokens
			let ingested_tokens = IngestedTokens {
				data: vec![text],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
				image_id: None,
			};
			yield Ok(ingested_tokens);

			if !slide_images.is_empty() {
				// Process images in parallel using Rayon and then spawn image ingestion
				let image_tasks: Vec<_> = slide_images.clone().into_par_iter().map(|(image_id, img_data)| {
					let file_path = PathBuf::from(file.clone());
					let doc_source = doc_source.clone();
					let source_id = source_id.clone();
					// Spawn blocking task for each image
					spawn_blocking(move || {
						let format = guess_format(&img_data);
						let mut ext = "jpeg";
						if let Ok(f) = format {
							ext = f.to_mime_type().split("/").last().unwrap_or("jpeg");
						}
						let collected_bytes = CollectedBytes {
							data: Some(Box::pin(std::io::Cursor::new(img_data.to_vec()))),
							file: Some(file_path),
							doc_source: Some(doc_source.clone()),
							eof: false,
							extension: Some(ext.to_string()),
							size: Some(img_data.len()),
							source_id: source_id.clone(),
							_owned_permit: None,
							image_id: Some(image_id.to_string()),
						};
						let image_ingestor = ImageIngestor::new();
						// Note: This is the blocking code, but the async function will return a Future.
						// Therefore, we return the Future itself here and handle it later in the async code.
						tokio::runtime::Handle::current().block_on(image_ingestor.ingest(vec![collected_bytes]))
					})
				}).collect();
				// Wait for all image tasks to complete and yield results
				for image_task in image_tasks {
					match image_task.await {
						Ok(Ok(image_stream)) => {
							let mut image_stream = Box::pin(image_stream);
							while let Some(token_result) = image_stream.next().await {
								yield token_result;
							}
						},
						Ok(Err(e)) => {
							eprintln!("Image ingestion failed: {:?}", e);
						},
						Err(join_err) => {
							eprintln!("Failed to process image: {:?}", join_err);
						}
					}
				}
			}
			// Final yield for empty token to signal end
			yield Ok(IngestedTokens {
				data: vec![],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
				image_id: None,
			});
		};

		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}

#[cfg(test)]
mod tests {
	use futures::StreamExt;

	use super::*;
	use std::{io::Cursor, path::Path};

	#[tokio::test]
	async fn test_odp_ingestor() {
		let included_bytes = include_bytes!("../../../../test_data/samplepptx.odp");
		let bytes = included_bytes.to_vec();

		// Create a CollectedBytes instance
		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("samplepptx.odp").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("odp".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		// Create an OdpIngestor instance
		let ingestor = OdpIngestor::new();

		// Ingest the file
		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();
		let mut has_found_image = false;
		let mut stream = result_stream;
		let mut all_data = Vec::new();
		while let Some(tokens) = stream.next().await {
			match tokens {
				Ok(tokens) =>
					if !tokens.data.is_empty() {
						if tokens.image_id.is_some() {
							has_found_image = true;
						}
						all_data.push(tokens.data);
					},
				Err(e) => {
					eprintln!("Failed to get tokens: {:?}", e);
				},
			}
		}
		assert!(all_data.len() >= 1, "Unable to ingest ODP file");
		assert!(has_found_image, "Unable to ingest images from ODP file");
	}
}
