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

// Define the OdpIngestor
pub struct OdpIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl OdpIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
	}
}

#[async_trait]
impl BaseIngestor for OdpIngestor {
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


						if file.name() == "content.xml" {
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

					let reader = EventReader::from_str(&xml_data);
					let mut text = String::new();
					let mut to_read = false;

					for e in reader {
						match e {
							Ok(XmlEvent::StartElement { name, .. }) => {
								if name.local_name == "p" || name.local_name == "span" {
									to_read = true;
								}
							}
							Ok(XmlEvent::Characters(data)) => {
								if to_read {
									text.push_str(&data);
								}
							}
							Ok(XmlEvent::EndElement { name }) => {
								if name.local_name == "p" {
									text.push_str("\n\n");
									to_read = false;
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
//     async fn test_odp_ingestor() {

//         let bytes = std::fs::read("/home/ansh/pyg-trail/odp/dummy.odp").unwrap();

//         // Create a CollectedBytes instance
//         let collected_bytes = CollectedBytes {
//             data: Some(bytes),
//             file: Some(Path::new("dummy.odp").to_path_buf()),
//             doc_source: Some("test_source".to_string()),
// 			eof: false,
// 			extension: Some("odp".to_string()),
// 			size: Some(10),
//          source_id: "Filesystem".to_string(),
//         };

//         let ingestor = OdpIngestor::new();

//         let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

// 		let mut stream = result_stream;
//         while let Some(tokens) = stream.next().await {
// 			let tokens = tokens.unwrap();
// 			println!("These are the tokens in file --------------{:?}", tokens);
// 		}
// 	}
// }
