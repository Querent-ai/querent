// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{Stream, StreamExt};
use image::guess_format;
use proto::semantics::IngestedTokens;
use std::{
	collections::HashMap,
	io::{Cursor, Read},
	path::PathBuf,
	pin::Pin,
	sync::Arc,
};
use tokio::io::AsyncReadExt;
use tracing::info;
use xml::reader::{EventReader, XmlEvent};
use zip::ZipArchive;

use crate::{
	image::image::ImageIngestor, process_ingested_tokens_stream,
	processors::text_processing::TextCleanupProcessor, AsyncProcessor, BaseIngestor,
	IngestorResult,
};

// Define the DocIngestor
pub struct DocIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl DocIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}
}

#[async_trait]
impl BaseIngestor for DocIngestor {
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
			let mut file_path: PathBuf = PathBuf::new();
			let mut doc_source = String::new();
			let mut source_id = String::new();
			for collected_bytes in all_collected_bytes {
				if collected_bytes.data.is_none() || collected_bytes.file.is_none() {
					continue;
				}
				if file.is_empty() {
					file_path = collected_bytes.file.as_ref().unwrap().clone();
					file = collected_bytes.file.as_ref().unwrap().to_string_lossy().to_string();
				}
				if doc_source.is_empty() {
					doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
				}
				if let Some(mut data) = collected_bytes.data {
					let mut buf = Vec::new();
					if let Err(e) = data.read_to_end(&mut buf).await {
						tracing::error!("Failed to read doc: {:?}", e);
						continue;
					}
					buffer.extend_from_slice(&buf);
				}
				source_id = collected_bytes.source_id.clone();
			}
			let cursor = Cursor::new(buffer);
			let mut archive = match ZipArchive::new(cursor) {
				Ok(archive) => archive,
				Err(_e) => {
					yield Ok(IngestedTokens {
						data: vec![],
						file: file.clone(),
						doc_source: doc_source.clone(),
						is_token_stream: false,
						source_id: source_id.clone(),
						image_id: None,
					});
					return;
				}
			};
			let mut xml_data = String::new();
			let mut images: HashMap<String, Vec<u8>> = HashMap::new();
			let file_name: String = file.clone();
			let mut tokens_to_yield: Vec<IngestedTokens> = Vec::new();
			for i in 0..archive.len() {
				let mut file = match archive.by_index(i) {
					Ok(file) => file,
					Err(_e) => {
						tokens_to_yield.push(IngestedTokens {
							data: vec![],
							file: file_name.clone(),
							doc_source: doc_source.clone(),
							is_token_stream: false,
							source_id: source_id.clone(),
							image_id: None,
						});
						continue;
					}
				};
				if file.name() == "word/document.xml" {
					if let Err(_e) = file.read_to_string(&mut xml_data) {
						tokens_to_yield.push(IngestedTokens {
							data: vec![],
							file: file_name.clone(),
							doc_source: doc_source.clone(),
							is_token_stream: false,
							source_id: source_id.clone(),
							image_id: None,
						});
						continue;
					}
				}

				if file.name().starts_with("word/media/") {
					let mut buf = Vec::new();
					if let Err(_e) = file.read_to_end(&mut buf) {
						tokens_to_yield.push(IngestedTokens {
							data: vec![],
							file: file_name.clone(),
							doc_source: doc_source.clone(),
							is_token_stream: false,
							source_id: source_id.clone(),
							image_id: None,
						});
						continue;
					}
					images.insert(file.name().to_string(), buf);
				}
			}
			for token in tokens_to_yield {
				yield Ok(token);
			}
			if xml_data.is_empty() {
				yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id: None,
				});
				return;
			}
			let reader = EventReader::from_str(&xml_data);
			let mut text = String::new();
			let mut to_read = false;
			for e in reader {
				match e {
					Ok(XmlEvent::StartElement { name, .. }) => {
						if name.local_name == "t" {
							to_read = true;
						}
					}
					Ok(XmlEvent::Characters(data)) => {
						if to_read {
							text.push_str(&data);
							to_read = false;
						}
					}
					Ok(XmlEvent::EndElement { name }) => {
						if name.local_name == "p" {
							text.push_str("\n\n");
						}
					}
					Err(_e) => {
						yield Ok(IngestedTokens {
								data: vec![],
								file: file.clone(),
								doc_source: doc_source.clone(),
								is_token_stream: false,
								source_id: source_id.clone(),
								image_id: None,
							});
						return;
					}
					_ => {}
				}
			}
			if text.is_empty() && images.is_empty() {
				info!("No content.xml found in the archive or the file is empty");
				return;
			}
			let ingested_tokens = IngestedTokens {
				data: vec![text],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
				image_id: None,
			};
			yield Ok(ingested_tokens);

			if !images.is_empty() {
				for (name, img_data) in images {
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
						image_id: Some(name.to_string()),
					};
					let image_ingestor = ImageIngestor::new();
					let (tx, mut rx) = tokio::sync::mpsc::channel(100);
					tokio::spawn(async move {
						let image_stream = image_ingestor.ingest(vec![collected_bytes]).await.unwrap();
						let mut image_stream = Box::pin(image_stream);
						while let Some(tokens) = image_stream.next().await {
							match tokens {
								Ok(tokens) => if !tokens.data.is_empty() {
									// only yield good tokens
									tx.send(Ok(tokens)).await.unwrap();
								},
								Err(e) => tracing::error!("Failed to get tokens from images: {:?}", e),
							}
						}
					});
					while let Some(tokens) = rx.recv().await {
						yield tokens;
					}
				}
			}
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
	async fn test_doc_ingestor() {
		let included_bytes = include_bytes!("../../../../test_data/Decline curve analysis of shale oil production_ The case of Eagle Ford.doc");
		let bytes = included_bytes.to_vec();

		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(
				Path::new(
					"Decline curve analysis of shale oil production_ The case of Eagle Ford.doc",
				)
				.to_path_buf(),
			),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("doc".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		let ingestor = DocIngestor::new();

		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();
		let mut has_found_image_data = false;
		let mut stream = result_stream;
		let mut all_data = Vec::new();
		while let Some(tokens) = stream.next().await {
			match tokens {
				Ok(tokens) =>
					if !tokens.data.is_empty() {
						if tokens.image_id.is_some() {
							has_found_image_data = true;
						}
						all_data.push(tokens.data);
					},
				Err(e) => {
					eprintln!("Failed to get tokens: {:?}", e);
				},
			}
		}
		assert!(all_data.len() >= 1, "Unable to ingest DOC file");
		assert!(has_found_image_data, "Not found image data in DOC file");
	}

	#[tokio::test]
	async fn test_doc_ingestor_with_corrupt_data() {
		let included_bytes = include_bytes!("../../../../test_data/corrupt-data/Demo.doc");
		let bytes = included_bytes.to_vec();

		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("Demo.doc").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("doc".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		let ingestor = DocIngestor::new();

		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();
		let mut stream = result_stream;
		let mut all_data = Vec::new();
		while let Some(tokens) = stream.next().await {
			match tokens {
				Ok(tokens) =>
					if !tokens.data.is_empty() {
						if tokens.image_id.is_some() {}
						all_data.push(tokens.data);
					},
				Err(e) => {
					eprintln!("Failed to get tokens: {:?}", e);
				},
			}
		}
		println!("Here");
		assert!(all_data.len() == 0, "Should not have ingested DOC file");
	}
}
