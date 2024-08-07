use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{io::Cursor, pin::Pin, sync::Arc};
use tokio::io::AsyncReadExt;

use crate::{process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorResult};
use xml::reader::{EventReader, XmlEvent};

// Define the TxtIngestor
pub struct XmlIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl XmlIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
	}
}

#[async_trait]
impl BaseIngestor for XmlIngestor {
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
			let cursor = Cursor::new(buffer);
			let parser = EventReader::new(cursor);
			let mut content = String::new();
			for e in parser {
				match e {
					Ok(XmlEvent::StartElement { name, .. }) => {
						content.push_str(&name.local_name);
						content.push_str("   ");
					},
					Ok(XmlEvent::Characters(data)) => {
						content.push_str(&data);
						content.push_str("\n");
					}
					Err(e) => {
						eprintln!("Error: {e}");
						break;
					}
					_ => {}
				}
			}
			let ingested_tokens = IngestedTokens {
				data: vec![content.to_string()],
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
	async fn test_xml_ingestor() {
		let included_bytes = include_bytes!("../../../../test_data/dc_universe.xml");
		let bytes = included_bytes.to_vec();

		// Create a CollectedBytes instance
		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("dc_universe.xml").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("xml".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
		};

		// Create a TxtIngestor instance
		let ingestor = XmlIngestor::new();

		// Ingest the file
		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

		let mut stream = result_stream;
		let mut all_data = Vec::new();
		while let Some(tokens) = stream.next().await {
			match tokens {
				Ok(tokens) => {
					if !tokens.data.is_empty() {
						all_data.push(tokens.data);
					}
				}
				Err(e) => {
					eprintln!("Failed to get tokens: {:?}", e);
				}
			}
		}
		assert!(all_data.len() >= 1, "Unable to ingest XML file");
	}
}
