use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{io::BufReader, AsyncReadExt, Stream};
use querent_synapse::comm::IngestedTokens;
use std::{pin::Pin, sync::Arc};

use crate::{
	process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorError, IngestorErrorKind,
	IngestorResult,
};

// Define the TxtIngestor
pub struct JsonIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl JsonIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
	}
}

#[async_trait]
impl BaseIngestor for JsonIngestor {
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

		let stream = stream! {
			let reader = BufReader::new(buffer.as_slice());
			let mut content = String::new();
			let mut buf_reader = BufReader::new(reader);

			// Read the entire content of the file
			buf_reader.read_to_string(&mut content).await
				.map_err(|err| IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())))?;

			let json: serde_json::Value = serde_json::from_str(&content).expect("JSON was not well-formatted");

			for (key, value) in json.as_object().expect("Failed to get object").iter() {
				let res = format!("{:?}   {:?}", key, value).to_string();

				let ingested_tokens = IngestedTokens {
					data: Some(vec![res.to_string()]),
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: Some(false),
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
//     async fn test_json_ingestor() {

//         let bytes = std::fs::read("/home/ansh/pyg-trail/dummy.json").unwrap();

//         // Create a CollectedBytes instance
//         let collected_bytes = CollectedBytes {
//             data: Some(bytes),
//             file: Some(Path::new("dummy.json").to_path_buf()),
//             doc_source: Some("test_source".to_string()),
// 			eof: false,
// 			extension: Some("json".to_string()),
// 			size: Some(10),
//         };

//         // Create a TxtIngestor instance
//         let ingestor = JsonIngestor::new();

//         // Ingest the file
//         let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

// 		let mut stream = result_stream;
//         while let Some(tokens) = stream.next().await {
// 			let tokens = tokens.unwrap();
// 			println!("These are the tokens in file --------------{:?}", tokens);
// 		}
// 	}
// }
