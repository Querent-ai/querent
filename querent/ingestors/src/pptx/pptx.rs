use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{Stream, StreamExt};
use image::guess_format;
use proto::semantics::IngestedTokens;
use std::{path::PathBuf, pin::Pin, sync::Arc};
use tokio::io::AsyncReadExt;

use crate::{
	image::image::ImageIngestor, pptx::parser::extract_text_and_images_from_pptx,
	process_ingested_tokens_stream, processors::text_processing::TextCleanupProcessor,
	AsyncProcessor, BaseIngestor, IngestorResult,
};

// Define the TxtIngestor
pub struct PptxIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl PptxIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}
}

#[async_trait]
impl BaseIngestor for PptxIngestor {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
		self.processors = processors;
	}

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
	{
		let stream = stream! {
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
		let text_result = extract_text_and_images_from_pptx(&buffer);
		match text_result {
			Ok((texts, images)) => {
				for text in texts {
					let ingested_tokens = IngestedTokens {
						data: vec![text],
						file: file.clone(),
						doc_source: doc_source.clone(),
						is_token_stream: false,
						source_id: source_id.clone(),
						image_id: None,
					};
					yield Ok(ingested_tokens);
				}
				if images.len() > 0 {
					for image in &images {
						let img_data = image.1;
						let image_id = image.0.clone();
						let file_path = PathBuf::from(file.clone());
						// Guess the format of the image data
						let format = guess_format(&img_data);
						let mut ext = "jpeg";
						if let Ok(f) = format {
							ext = f.to_mime_type();
							ext = ext.split("/").last().unwrap_or("jpeg");
						}
						let collected_bytes = CollectedBytes {
							data: Some(Box::pin(std::io::Cursor::new(img_data.clone()))),
							file: Some(file_path.clone()),
							doc_source: Some(doc_source.clone()),
							eof: false,
							extension: Some(ext.to_string()),
							size: Some(img_data.len()),
							source_id: source_id.clone(),
							_owned_permit: None,
							image_id: Some(image_id.to_string()),
						};
						let image_ingestor = ImageIngestor::new();
						let image_stream = image_ingestor.ingest(vec![collected_bytes]).await.unwrap();
						let mut image_stream = Box::pin(image_stream);
						while let Some(tokens) = image_stream.next().await {
							match tokens {
								Ok(tokens) =>
									if !tokens.data.is_empty() {
										// only yield good tokens
										yield Ok(tokens);
									},
								Err(e) => {
									tracing::error!("Failed to get tokens from images: {:?}", e);
								},
							}
						}
					}
				}
			},
			Err(e) => {
				eprintln!("Error: {:?}", e);
				yield Err(e);
			}
		}
		yield Ok(IngestedTokens {
			data: vec![],
			file: file.clone(),
			doc_source: doc_source.clone(),
			is_token_stream: false,
			source_id: source_id.clone(),
			image_id: None,
		})
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
	async fn test_pptx_ingestor() {
		let included_bytes = include_bytes!("../../../../test_data/samplepptx.pptx");
		let bytes = included_bytes.to_vec();

		// Create a CollectedBytes instance
		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("samplepptx.pptx").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("pptx".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		// Create a TxtIngestor instance
		let ingestor = PptxIngestor::new();

		// Ingest the file
		let result_stream: Pin<
			Box<dyn Stream<Item = Result<IngestedTokens, crate::IngestorError>> + Send>,
		> = ingestor.ingest(vec![collected_bytes]).await.unwrap();
		let mut is_image_included = false;
		let mut stream = result_stream;
		let mut all_data = Vec::new();
		while let Some(tokens) = stream.next().await {
			match tokens {
				Ok(tokens) =>
					if !tokens.data.is_empty() {
						if tokens.image_id.is_some() {
							is_image_included = true;
						}
						all_data.push(tokens.data);
					},
				Err(e) => {
					eprintln!("Failed to get tokens: {:?}", e);
				},
			}
		}
		assert!(all_data.len() >= 1, "Unable to ingest DOC file");
		assert!(is_image_included, "Image not included in the ingestor output");
	}
}
