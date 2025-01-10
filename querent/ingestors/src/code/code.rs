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

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{io::Cursor, pin::Pin, sync::Arc};
use tokio::io::AsyncReadExt;

use crate::{
	process_ingested_tokens_stream, processors::text_processing::TextCleanupProcessor,
	AsyncProcessor, BaseIngestor, IngestorResult,
};

// Define the TxtIngestor
pub struct CodeIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl CodeIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}
}

#[async_trait]
impl BaseIngestor for CodeIngestor {
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
					let read_res = data.read_to_end(&mut buf).await;
					if read_res.is_err() {
						yield Ok(IngestedTokens {
							data: vec![],
							file: file.clone(),
							doc_source: doc_source.clone(),
							is_token_stream: false,
							source_id: source_id.clone(),
							image_id: None,
						})
						}
					buffer.extend_from_slice(&buf);
				}
				source_id = collected_bytes.source_id.clone();
				}

				let mut content = String::new();
				let mut cursor = Cursor::new(buffer);
				let is_read = cursor.read_to_string(&mut content).await;
				if is_read.is_err() {
					yield Ok(IngestedTokens {
						data: vec![],
						file: file.clone(),
						doc_source: doc_source.clone(),
						is_token_stream: false,
						source_id: source_id.clone(),
						image_id: None,
					})
				}else {
				let ingested_tokens = IngestedTokens {
					data: vec![content.to_string()],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id: None,
				};
				yield Ok(ingested_tokens);

				let ingested_tokens = IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id: None,
				};

				yield Ok(ingested_tokens);
			}
		};

		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use futures::StreamExt;
	use std::path::Path;

	#[tokio::test]
	async fn test_code_ingestor() {
		let included_bytes = include_bytes!("../csv/csv.rs");
		let bytes = included_bytes.to_vec();

		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("csv.rs").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("rs".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		let ingestor = CodeIngestor::new();

		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

		let mut stream = result_stream;
		let mut found_data = false;
		while let Some(tokens) = stream.next().await {
			match tokens {
				Ok(tokens_res) =>
					if !tokens_res.data.is_empty() {
						found_data = true;
					},
				Err(e) => {
					eprintln!("Found error as {:?}", e);
				},
			}
		}

		assert!(found_data, "Data not found");
	}

	#[tokio::test]
	async fn test_code_ingestor_with_corrupt_file() {
		let included_bytes = include_bytes!("../../../../test_data/corrupt-data/Demo.rs");
		let bytes = included_bytes.to_vec();

		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("Demo.rs").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("rs".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		let ingestor = CodeIngestor::new();

		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

		let mut stream = result_stream;
		let mut found_data = false;
		while let Some(tokens) = stream.next().await {
			match tokens {
				Ok(tokens_res) =>
					if !tokens_res.data.is_empty() {
						found_data = true;
					},
				Err(e) => {
					eprintln!("Found error as {:?}", e);
				},
			}
		}
		assert!(!found_data, "Data should not have been found");
	}
}
