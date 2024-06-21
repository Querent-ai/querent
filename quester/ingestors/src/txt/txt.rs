use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{pin::Pin, sync::Arc};
use tokio::io::{AsyncReadExt, BufReader};

use crate::{
	process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorError, IngestorErrorKind,
	IngestorResult,
};

// Define the TxtIngestor
pub struct TxtIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl TxtIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
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
		// collect all the bytes into a single buffer
		let mut buffer = Vec::new();
		let mut file = String::new();
		let mut doc_source = String::new();
		for collected_bytes in all_collected_bytes.iter() {
			if file.is_empty() {
				file =
					collected_bytes.clone().file.unwrap_or_default().to_string_lossy().to_string();
			}
			if doc_source.is_empty() {
				doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
			}
			buffer.extend_from_slice(&collected_bytes.clone().data.unwrap_or_default());
		}

		// Create a stream to read the text file content
		let stream = stream! {
			let reader = BufReader::new(buffer.as_slice());
			let mut content = String::new();
			let mut buf_reader = BufReader::new(reader);

			// Read the entire content of the file
			buf_reader.read_to_string(&mut content).await
				.map_err(|err| IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())))?;

			let ingested_tokens = IngestedTokens {
				data: vec![content],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
			};

			yield Ok(ingested_tokens);
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
	use std::{fs::File, io::Write};
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
			data: Some(bytes),
			file: Some(test_file_path.into()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("txt".to_string()),
			size: Some(10),
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
			println!("These are the tokens in file --------------{:?}", tokens);
			count += 1;
		}
		assert_eq!(count, 1);
	}
}
