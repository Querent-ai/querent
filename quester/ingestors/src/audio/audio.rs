use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{io::Cursor, pin::Pin, sync::Arc};

use crate::{process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorResult};

// Define the TxtIngestor
pub struct AudioIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl AudioIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
	}
}

#[async_trait]
impl BaseIngestor for AudioIngestor {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
		self.processors = processors;
	}

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
	{
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

		let buffer = Arc::new(buffer);
		let stream = {
			let buffer = Arc::clone(&buffer);
			stream! {
			let _cursor = Cursor::new(buffer.as_ref());
			// TODO: find a library capable of converting audio data to text format
			let ingested_tokens = IngestedTokens {
				data: vec!["".to_string()],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
			};

			yield Ok(ingested_tokens);
			}
		};

		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}
