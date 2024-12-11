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
use image::guess_format;
use tokio::io::AsyncReadExt as _;
use tracing::{error, info};

use futures::{Stream, StreamExt};
use proto::semantics::IngestedTokens;
use std::{
	collections::HashMap,
	io::{Cursor, Read},
	path::PathBuf,
	pin::Pin,
	sync::Arc,
};

use crate::{
	image::image::ImageIngestor, process_ingested_tokens_stream,
	processors::text_processing::TextCleanupProcessor, AsyncProcessor, BaseIngestor,
	IngestorResult,
};
use xml::{reader::XmlEvent, EventReader};
use zip::ZipArchive;

// Define the DocxIngestor
pub struct DocxIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl DocxIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}
}

#[async_trait]
impl BaseIngestor for DocxIngestor {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
		self.processors = processors;
	}

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
	{
		let stream = stream! {
			// collect all the bytes into a single buffer
			let mut buffer = Vec::new();
			let mut file_path = PathBuf::new();
			let mut file = String::new();
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
						tracing::error!("Failed to read docx: {:?}", e);
						continue;
					}
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
			let mut images: HashMap<String, Vec<u8>> = HashMap::new();
			for i in 0..archive.len() {
				let mut file = match archive.by_index(i) {
					Ok(file) => file,
					Err(e) => {
						error!("Failed to access file in archive: {:?}", e);
						return;
					}
				};
				if file.name() == "word/document.xml" {
					if let Err(e) = file.read_to_string(&mut xml_data) {
						error!("Failed to read XML data: {:?}", e);
						return;
					}
				}

				if file.name().starts_with("word/media/") {
					let mut buf = Vec::new();
					if let Err(e) = file.read_to_end(&mut buf) {
						error!("Failed to read image data: {:?}", e);
						return;
					}
					images.insert(file.name().to_string(), buf);
				}
			}

			if xml_data.is_empty() && images.is_empty() {
				info!("No content.xml found in the archive or the file is empty");
				return;
			}

			let parser = EventReader::from_str(&xml_data);
			let mut txt: Vec<String> = Vec::new();
			let mut in_text = false;

			for event in parser {
				match event {
					Ok(XmlEvent::StartElement { name, .. }) => {
						if name.local_name == "p" {
							txt.push("\n".to_string());
						} else if name.local_name == "t" {
							in_text = true;
						}
					}
					Ok(XmlEvent::Characters(content)) if in_text => {
						txt.push(content);
						in_text = false;
					}
					Ok(XmlEvent::EndDocument) => break,
					Err(e) => {
						error!("Error reading XML: {:?}", e);
						return;
					}
					_ => {}
				}
			}

			for text in txt {
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
	async fn test_docx_ingestor() {
		let included_bytes = include_bytes!("../../../../test_data/sample-document.docx");
		let bytes = included_bytes.to_vec();

		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("sample-document.docx").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("docx".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		let ingestor = DocxIngestor::new();

		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();
		let mut found_image_data = false;
		let mut stream = result_stream;
		let mut all_data = Vec::new();
		while let Some(tokens) = stream.next().await {
			match tokens {
				Ok(tokens) =>
					if !tokens.data.is_empty() {
						if tokens.image_id.is_some() {
							found_image_data = true;
						}
						all_data.push(tokens.data);
					},
				Err(e) => {
					eprintln!("Failed to get tokens: {:?}", e);
				},
			}
		}
		assert!(all_data.len() >= 1, "Unable to ingest DOCX file");
		assert!(found_image_data, "Unable to ingest image data");
	}

	#[tokio::test]
	async fn test_docx_ingestor_with_corrupt_file() {
		let included_bytes = include_bytes!("../../../../test_data/corrupt-data/Demo.docx");
		let bytes = included_bytes.to_vec();

		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("Demo.docx").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("docx".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		let ingestor = DocxIngestor::new();

		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();
		let mut stream = result_stream;
		let mut all_data = Vec::new();
		while let Some(tokens) = stream.next().await {
			match tokens {
				Ok(tokens) =>
					if !tokens.data.is_empty() {
						all_data.push(tokens.data);
					},
				Err(e) => {
					eprintln!("Failed to get tokens: {:?}", e);
				},
			}
		}
		assert!(all_data.len() == 0, "Should not have found data in DOCX file");
	}
}
