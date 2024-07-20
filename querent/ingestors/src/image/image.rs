use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use rusty_tesseract::{image, Image};
use std::{pin::Pin, sync::Arc};

use crate::{process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorResult};

// Define the TxtIngestor
pub struct ImageIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl ImageIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
	}
}

#[async_trait]
impl BaseIngestor for ImageIngestor {
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

		let stream = {
			stream! {
			let dyn_img = image::load_from_memory(&buffer).unwrap();

			let img = Image::from_dynamic_image(&dyn_img).unwrap();

			let default_args = rusty_tesseract::Args::default();

			let output = rusty_tesseract::image_to_string(&img, &default_args).unwrap();

			let ingested_tokens = IngestedTokens {
				data: vec![output.to_string()],
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
//     async fn test_img_ingestor() {

//         let bytes = std::fs::read("/home/ansh/pyg-trail/1520206936825.jpeg").unwrap();

//         // Create a CollectedBytes instance
//         let collected_bytes = CollectedBytes {
//             data: Some(bytes),
//             file: Some(Path::new("1520206936825.jpeg").to_path_buf()),
//             doc_source: Some("test_source".to_string()),
// 			eof: false,
// 			extension: Some("jpeg".to_string()),
// 			size: Some(10),
//          source_id: "Filesystem".to_string(),
//         };

//         // Create a TxtIngestor instance
//         let ingestor = ImageIngestor::new();

//         // Ingest the file
//         let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

// 		let mut stream = result_stream;
//         while let Some(tokens) = stream.next().await {
// 			let tokens = tokens.unwrap();
// 			println!("These are the tokens in file --------------{:?}", tokens);
// 		}
// 	}
// }
