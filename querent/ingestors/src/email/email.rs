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

// Define the EmailsIngestor
pub struct EmailIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl EmailIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
	}
}

#[async_trait]
impl BaseIngestor for EmailIngestor {
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

			let reader = BufReader::new(buffer.as_slice());
			let mut content = String::new();
			let mut buf_reader = BufReader::new(reader);

			buf_reader.read_to_string(&mut content).await
				.map_err(|err| IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())))?;

			let ingested_tokens = IngestedTokens {
				data: vec![content],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
			};

			yield Ok(ingested_tokens);


			yield Ok(IngestedTokens {
				data: vec![],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
			})
		};

		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}
