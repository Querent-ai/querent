use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{pin::Pin, sync::Arc};

use crate::{
	pptx::parser::extract_text_from_pptx, process_ingested_tokens_stream, AsyncProcessor,
	BaseIngestor, IngestorResult,
};

// Define the TxtIngestor
pub struct PptxIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl PptxIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
	}
}

#[async_trait]
impl BaseIngestor for PptxIngestor {
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

		let stream = {
			stream! {
				let text_result = extract_text_from_pptx(&buffer);
				match text_result {
					Ok(text) => {
						let ingested_tokens = IngestedTokens {
							data: vec![text],
							file: file.clone(),
							doc_source: doc_source.clone(),
							is_token_stream: false,
							source_id: source_id.clone(),
						};
						yield Ok(ingested_tokens);
					},
					Err(e) => {
						eprintln!("Error: {:?}", e);
						yield Err(e);
					}
				}
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
//     async fn test_pptx_ingestor() {

//         let bytes = std::fs::read("/home/ansh/pyg-trail/ppt/Eagle-Ford-Shale-Basin-Study-APR_22_2019.pptx").unwrap();

//         // Create a CollectedBytes instance
//         let collected_bytes = CollectedBytes {
//             data: Some(bytes),
//             file: Some(Path::new("Eagle-Ford-Shale-Basin-Study-APR_22_2019.pptx").to_path_buf()),
//             doc_source: Some("test_source".to_string()),
// 			eof: false,
// 			extension: Some("pptx".to_string()),
// 			size: Some(10),
//          source_id: "Filesystem".to_string(),
//         };

//         // Create a TxtIngestor instance
//         let ingestor = PptxIngestor::new();

//         // Ingest the file
//         let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

// 		let mut stream = result_stream;
//         while let Some(tokens) = stream.next().await {
// 			let tokens = tokens.unwrap();
// 			println!("These are the tokens in file --------------{:?}", tokens);
// 		}
// 	}
// }
