use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use docx_rust::{
	document::{BodyContent, ParagraphContent, RunContent},
	DocxFile,
};
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{io::Cursor, pin::Pin, sync::Arc};

use crate::{process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorResult};

// Define the DocxIngestor
pub struct DocxIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl DocxIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
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
		let stream = {
			stream! {
				// collect all the bytes into a single buffer
				let mut buffer = Vec::new();
				let mut file = String::new();
				let mut doc_source = String::new();
				let mut source_id = String::new();
				for collected_bytes in all_collected_bytes.iter() {
					if collected_bytes.data.is_none() || collected_bytes.file.is_none() {
						continue;
					}
					if file.is_empty() {
						file = collected_bytes.file.as_ref().unwrap().to_string_lossy().to_string();
					}
					if doc_source.is_empty() {
						doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
					}
					buffer.extend_from_slice(collected_bytes.data.as_ref().unwrap().as_slice());
					source_id = collected_bytes.source_id.clone();
				}
				let cursor = Cursor::new(buffer);
				let docx = DocxFile::from_reader(cursor).unwrap();
				let docx = match docx.parse() {
					Ok(parsed) => parsed,
					Err(e) => {
						eprintln!("Failed to parse DOCX: {:?}", e);
						return;
					}
				};
				let body = docx.document.body;
				for content_item in body.content {
					let mut text = String::new();
					match content_item {
						BodyContent::Paragraph(paragraph) => {
							for content in paragraph.content {
								match content {
									ParagraphContent::Run(run) => {
										for run_content in run.content {
											match run_content {
												RunContent::Text(text_struct) => {
													text.push_str(&text_struct.text);
												},
												_ => {}
											}
										}
									},
									_ => {}
								}
							}
						},
						BodyContent::Table(_table) => {
							// TODO handle table
						},
						BodyContent::Sdt(_sdt) => {
							// TODO handle Sdt
						},
						BodyContent::SectionProperty(_section) => {
							// TODO handle section property
						},
						BodyContent::TableCell(_cell) => {
							// TODO handle cell
						},
					}
					if text == "" {
						continue;
					}
					let ingested_tokens = IngestedTokens {
						data: vec![text],
						file: file.clone(),
						doc_source: doc_source.clone(),
						is_token_stream: false,
						source_id: source_id.clone(),
					};
					yield Ok(ingested_tokens);
				}

				yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
				})
			}
		};

		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}
