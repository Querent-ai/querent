use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{pin::Pin, sync::Arc};
use tokio::io::{AsyncReadExt, BufReader};

use crate::{
	process_ingested_tokens_stream, processors::text_processing::TextCleanupProcessor,
	AsyncProcessor, BaseIngestor, IngestorResult,
};

// Define the TxtIngestor
pub struct TxtIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl TxtIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}
}

#[async_trait]
impl BaseIngestor for TxtIngestor {
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
			// Create a stream to read the text file content
			let reader = BufReader::new(buffer.as_slice());
			let mut content = String::new();
			let mut buf_reader = BufReader::new(reader);
			// Read the entire content of the file
			let is_read = buf_reader.read_to_string(&mut content).await;
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
					data: vec![content],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id: None,
				};
				yield Ok(ingested_tokens);
				yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id: None,
				})
			}
		};

		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}

// tests/txt_ingestor_tests.rs
#[cfg(test)]
mod tests {
	use super::*;
	use futures::StreamExt;
	use std::{
		fs::File,
		io::{Cursor, Write},
	};
	use tokio::fs::read;

	#[tokio::test]
	async fn test_txt_ingestor() {
		// Create a sample .txt file for testing
		let test_file_path = "/tmp/test_sample.txt";
		let mut file = File::create(test_file_path).expect("Failed to create test file");
		writeln!(file, "This is a test file.").expect("Failed to write to test file");

		// Read the sample .txt file
		let bytes = read(test_file_path).await.expect("Failed to read test file");
		// Create a CollectedBytes instance
		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(test_file_path.into()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("txt".to_string()),
			size: Some(10),
			source_id: "Filesystem".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		// Create a TxtIngestor instance
		let ingestor = TxtIngestor::new();

		// Ingest the file
		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

		// Collect the stream into a Vec
		let mut count = 0;
		let mut stream = result_stream;
		while let Some(tokens) = stream.next().await {
			let tokens = tokens.unwrap();
			if tokens.data.len() > 0 {
				count += 1;
			}
		}
		assert_eq!(count, 1);
	}

	#[tokio::test]
	async fn test_txt_ingestor_non_utf8() {
		let included_bytes = include_bytes!("../../../../test_data/fontawesome-webfont.bin");
		let bytes = included_bytes.to_vec();
		// Create a CollectedBytes instance
		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some("path".into()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("".to_string()),
			size: Some(10),
			source_id: "Filesystem".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		// Create a TxtIngestor instance
		let ingestor = TxtIngestor::new();

		// Ingest the file
		let result_stream = ingestor.ingest(vec![collected_bytes]).await;
		assert!(result_stream.is_ok());
		// The result stream should be empty
		let mut stream = result_stream.unwrap();
		let mut count = 0;
		while let Some(tokens) = stream.next().await {
			let tokens = tokens.unwrap();
			if tokens.data.len() > 0 {
				count += 1;
			}
		}
		assert_eq!(count, 0);
	}
}
