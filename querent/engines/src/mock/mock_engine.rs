// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use crate::{Engine, EngineResult};
use async_stream::stream;
use async_trait::async_trait;
use common::{EventState, EventType, SemanticKnowledgePayload};
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::pin::Pin;
use tokio::sync::mpsc::Receiver;

pub struct MockEngine;

#[async_trait]
impl Engine for MockEngine {
	async fn process_ingested_tokens<'life0>(
		&'life0 self,
		token_stream: Receiver<IngestedTokens>,
	) -> EngineResult<Pin<Box<dyn Stream<Item = EngineResult<EventState>> + Send + 'life0>>> {
		let stream = stream! {
			let mut token_stream = token_stream;
			while let Some(token) = token_stream.recv().await {
				// create a payload
				let payload = SemanticKnowledgePayload {
					subject: "mock".to_string(),
					subject_type: "mock".to_string(),
					predicate: "mock".to_string(),
					predicate_type: "mock".to_string(),
					object: "mock".to_string(),
					object_type: "mock".to_string(),
					sentence: "mock".to_string(),
					image_id: None,
					blob: Some("mock".to_string()),
					event_id: "mock".to_string(),
					source_id: "mock".to_string(),
				};

				// create an event
				let event = EventState {
					event_type: EventType::Graph,
					file: token.file,
					doc_source: token.doc_source,
					image_id: None,
					timestamp: 0.0,
					payload: serde_json::to_string(&payload).unwrap_or_default(),
				};
				yield Ok(event);
			}
		};
		Ok(Box::pin(stream))
	}
}

impl MockEngine {
	pub fn new() -> Self {
		MockEngine
	}
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use futures::StreamExt;
//     use std::{fs::File, io::Write};
//     use tokio::fs::read;
//     use crossbeam_channel::bounded;
//     use ingestors::{txt::txt::TxtIngestor, BaseIngestor};
//     use common::CollectedBytes;

//     #[tokio::test]
//     async fn test_txt_ingestor() {
//         // Create a sample .txt file for testing
//         let test_file_path = "/tmp/test_sample.txt";
//         let mut file = File::create(test_file_path).expect("Failed to create test file");
//         writeln!(file, "This is a test file.").expect("Failed to write to test file");

//         // Read the sample .txt file
//         let bytes = read("/home/nishantg/querent-main/querent/text_test".to_string()).await.expect("Failed to read test file");

//         // Create a CollectedBytes instance
//         let collected_bytes = CollectedBytes {
//             data: Some(bytes.to_vec()), // Convert the byte slice to a Vec<u8>
//             file: Some(test_file_path.into()),
//             doc_source: Some("test_source".to_string()),
//             eof: false,
//             extension: Some("txt".to_string()),
//             size: Some(10),
//         };

//         // Create a TxtIngestor instance
//         let ingestor = TxtIngestor::new();

//         // Ingest the file
// 		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();
//         // let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

//         // Create a crossbeam channel with a bounded capacity
//         let (sender, receiver) = bounded(10);

//         // Send the tokens to the channel
//         let mut stream = result_stream;
//         while let Some(tokens) = stream.next().await {
//             let tokens = tokens.unwrap();
//             println!("These are the tokens in file --------------{:?}", tokens);

//             // Send the IngestedTokens through the channel
//             sender.send(tokens).unwrap();
//         }

//         // Close the sender to indicate no more tokens will be sent
//         drop(sender);

//         // Create an instance of MockEngine
//         let engine = MockEngine::new();

//         // Process the ingested tokens
//         let mut engine_stream = engine.process_ingested_tokens(receiver).await.unwrap();

//         // Collect the results from the stream
//         let mut results = Vec::new();
//         while let Some(result) = engine_stream.next().await {
//             results.push(result.unwrap());
//         }

//         // Verify the results
//         assert_eq!(results.len(), 1); // Adjust based on your expectations
//         assert_eq!(results[0].file, test_file_path);
//         assert_eq!(results[0].doc_source, "test_source");
//     }
// }
