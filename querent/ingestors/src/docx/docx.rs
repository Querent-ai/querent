use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use tokio::io::AsyncReadExt as _;
use tracing::error;

use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{
	io::{Cursor, Read},
	pin::Pin,
	sync::Arc,
};

use crate::{process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorResult};
use xml::{reader::XmlEvent, EventReader};
use zip::ZipArchive;

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
					data.read_to_end(&mut buf).await.unwrap();
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
					break;
				}
			}

			if xml_data.is_empty() {
				error!("No content.xml found in the archive or the file is empty");
				return;
			}

			let parser = EventReader::from_str(&xml_data);
			let mut txt = Vec::new();
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

			yield Ok(IngestedTokens {
				data: vec![txt.join("")],
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::path::Path;
//     use futures::StreamExt;
// 	use common::OwnedBytes;

//     #[tokio::test]
//     async fn test_docx_ingestor() {

//         let bytes = std::fs::read("/home/ansh/pyg-trail/doc/Decline curve analysis of shale oil production_ The case of Eagle Ford.docx").unwrap();

//         // Create a CollectedBytes instance
//         let collected_bytes = CollectedBytes {
//             data: Some(OwnedBytes::new(bytes)),
//             file: Some(Path::new("Decline curve analysis of shale oil production_ The case of Eagle Ford.docx").to_path_buf()),
//             doc_source: Some("test_source".to_string()),
//             eof: false,
//             extension: Some("docx".to_string()),
//             size: Some(10),
//             source_id: "Filesystem".to_string(),
//         };

//         // Create a HtmlIngestor instance
//         let ingestor = DocxIngestor::new();

//         // Ingest the file
//         let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

//         let mut stream = result_stream;
//         while let Some(tokens) = stream.next().await {
//             let tokens = tokens.unwrap();
//             println!("These are the tokens in file --------------{:?}", tokens);
//         }
//     }
// }
