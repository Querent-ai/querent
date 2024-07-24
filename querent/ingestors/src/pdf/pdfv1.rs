use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, pin::Pin, sync::Arc};

use crate::{
	process_ingested_tokens_stream, processors::text_processing::TextCleanupProcessor,
	AsyncProcessor, BaseIngestor, IngestorError, IngestorErrorKind, IngestorResult,
};

#[derive(Debug, Deserialize, Serialize)]
struct PdfText {
	text: BTreeMap<u32, Vec<String>>, // Key is page number
	errors: Vec<String>,
}

// Define the PdfIngestor
pub struct PdfIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl PdfIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}
}

#[async_trait]
impl BaseIngestor for PdfIngestor {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
		self.processors = processors;
	}

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
	{
		// collect all the bytes into a single buffer
		let stream = stream! {
			let mut buffer = Vec::new();
			let mut file = String::new();
			let mut doc_source = String::new();
			let mut source_id = String::new();
			for collected_bytes in all_collected_bytes.iter() {
				if file.is_empty() {
					file =
						collected_bytes.clone().file.unwrap_or_default().to_string_lossy().to_string();
				}
				if doc_source.is_empty() {
					doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
				}
				buffer.extend_from_slice(&collected_bytes.clone().data.unwrap_or_default());
				source_id = collected_bytes.source_id.clone();
			}
			let doc = lopdf::Document::load_mem(&buffer);
			if let Err(e) = doc {
				yield Err(IngestorError::new(IngestorErrorKind::Internal, Arc::new(e.into())));
				return;
			}
			let mut pdf_text: PdfText = PdfText {
				text: BTreeMap::new(),
				errors: Vec::new(),
			};
			let doc = doc.unwrap();

			let pages: Vec<Result<(u32, Vec<String>), IngestorError>> = doc
				.get_pages()
				.into_par_iter()
				.map(
					|(page_num, _page_id): (u32, (u32, u16))| -> Result<(u32, Vec<String>), IngestorError> {
						let text = doc.extract_text(&[page_num]).map_err(|e| {
							IngestorError::new(
								IngestorErrorKind::Internal,
								Arc::new(e.into()),
							)
						})?;
						Ok((
							page_num,
							text.split('\n')
								.map(|s| s.trim_end().to_string())
								.collect::<Vec<String>>(),
						))
					},
				)
				.collect();
			for page in pages {
				match page {
					Ok((page_num, lines)) => {
						pdf_text.text.insert(page_num, lines);
					}
					Err(e) => {
						pdf_text.errors.push(e.to_string());
					}
				}
			}
			let t = serde_json::to_string_pretty(&pdf_text).unwrap();

			let ingested_tokens = IngestedTokens {
				data: vec![t],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
			};

			yield Ok(ingested_tokens);
		};

		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}
