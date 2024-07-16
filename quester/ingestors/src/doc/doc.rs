use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{
	io::{Cursor, Read},
	pin::Pin,
	sync::Arc,
};
use tracing::error;
use xml::reader::{EventReader, XmlEvent};
use zip::ZipArchive;

use crate::{process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorResult};

// Define the DocIngestor
pub struct DocIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl DocIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
	}
}

#[async_trait]
impl BaseIngestor for DocIngestor {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
		self.processors = processors;
	}

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
	{
		// collect all the bytes into a single buffer
		println!("Inside ingest function: ");
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

		let buffer = Arc::new(buffer);
		let stream = {
			let buffer = Arc::clone(&buffer);
			stream! {
				let cursor = Cursor::new(buffer.as_ref());

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
					error!("No document.xml found in the archive or the file is empty");
					return;
				}

				let reader = EventReader::from_str(&xml_data);
				let mut text = String::new();
				let mut to_read = false;

				for e in reader {
					match e {
						Ok(XmlEvent::StartElement { name, .. }) => {
							if name.local_name == "t" {
								to_read = true;
							}
						}
						Ok(XmlEvent::Characters(data)) => {
							if to_read {
								text.push_str(&data);
								to_read = false;
							}
						}
						Ok(XmlEvent::EndElement { name }) => {
							if name.local_name == "p" {
								text.push_str("\n\n");
							}
						}
						Err(e) => {
							error!("Error reading XML: {:?}", e);
							return;
						}
						_ => {}
					}
				}

				if text.is_empty() {
					error!("No text found in the document");
					return;
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
// 	use futures::StreamExt;

//     #[tokio::test]
//     async fn test_csv_ingestor() {

//         let bytes = std::fs::read("/home/ansh/pyg-trail/doc/Eagle Ford Shale variability_ Sedimentologic influences on source and reservoir character in an unconventional resource unit.doc").unwrap();

//         // Create a CollectedBytes instance
//         let collected_bytes = CollectedBytes {
//             data: Some(bytes),
//             file: Some(Path::new("Eagle Ford Shale variability_ Sedimentologic influences on source and reservoir character in an unconventional resource unit.doc").to_path_buf()),
//             doc_source: Some("test_source".to_string()),
// 			eof: false,
// 			extension: Some("doc".to_string()),
// 			size: Some(10),
//          source_id: "Filesystem".to_string(),
//         };

//         let ingestor = DocIngestor::new();

//         let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

// 		let mut stream = result_stream;
//         while let Some(tokens) = stream.next().await {
// 			let tokens = tokens.unwrap();
// 			println!("These are the tokens in file --------------{:?}", tokens);
// 		}
// 	}
// }
