use crate::utils::{
	add_attention_to_classified_sentences, create_binary_pairs, label_entities_in_sentences, match_entities_with_tokens, remove_newlines, split_into_chunks, tokens_to_words, ClassifiedSentence
};
use async_stream::stream;
use async_trait::async_trait;
use common::{EventState, EventType, SemanticKnowledgePayload};
use futures::{Stream, StreamExt};
use llms::llm::LLM;
use proto::semantics::IngestedTokens;
use std::{pin::Pin, sync::Arc};
use candle_core::{DType, Tensor, Device};

use crate::{Engine, EngineError, EngineResult};

pub struct AttentionTensorsEngine {
	pub llm: Arc<dyn LLM>,
	pub entities: Vec<String>,
}

impl AttentionTensorsEngine {
	pub fn new(llm: Arc<dyn LLM>, entities: Vec<String>) -> Self {
		Self { llm, entities }
	}
}

#[async_trait]
impl Engine for AttentionTensorsEngine {
	async fn process_ingested_tokens(
		&self,
		token_stream: Pin<Box<dyn Stream<Item = IngestedTokens> + Send + 'static>>,
	) -> EngineResult<Pin<Box<dyn Stream<Item = EngineResult<EventState>> + Send + 'static>>> {
		// Await the maximum tokens value outside the stream! block
		let max_tokens = self.llm.maximum_tokens().await;

		// Prepare a vector to store tokens and chunks
		let mut all_chunks = Vec::new();

		// Process the token stream to get chunks
		let mut token_stream = token_stream;
		while let Some(token) = token_stream.next().await {

			// Process the data to remove '\n' characters using the utility function
			let cleaned_data: Vec<String> =
				token.data.into_iter().map(|s| remove_newlines(&s)).collect();

			// Split the cleaned data into chunks based on the maximum token length
			for data in cleaned_data {
				let split_chunks = split_into_chunks(max_tokens, &data);
				all_chunks.extend(split_chunks);
			}
		}
		// Tokenize each chunk
		let mut tokenized_chunks = Vec::new();
		for chunk in &all_chunks {
			let tokenized_chunk =
				self.llm.tokenize(chunk).await.map_err(|e| EngineError::from(e))?;
			tokenized_chunks.push(tokenized_chunk);
		}

		println!("Tokenized Chunks-----{:?}", tokenized_chunks);
		// Match entities with tokens and get their indices
		let classified_sentences = if !self.entities.is_empty() {
			println!("Using user-provided entities.");
			// Use the utility function to label entities in sentences
			let initial_classified_sentences =
				label_entities_in_sentences(&self.entities, &all_chunks);

			// Convert tokenized sequences into words
			let tokenized_words = tokens_to_words(self.llm.as_ref(), &tokenized_chunks).await;

			// Match entities with tokens and get their indices
			match_entities_with_tokens(&tokenized_words, &initial_classified_sentences)
		} else {
			// Convert tokenized sequences into model input format
			let mut model_inputs = Vec::new();
			for tokens in &tokenized_chunks {
				let model_input =
					self.llm.model_input(tokens.clone()).await.map_err(|e| EngineError::from(e))?;
				model_inputs.push(model_input);
			}

			// Call the token classification function
			let mut classification_results = Vec::new();
			for input in &model_inputs {
				let classification_result = self
					.llm
					.token_classification(input.clone(), None)
					.await
					.map_err(|e| EngineError::from(e))?;
				classification_results.push(classification_result);
			}

			// Combine sentences with their classifications
			let mut classified_sentences = Vec::new();
			for (chunk, classification) in all_chunks.iter().zip(classification_results.iter()) {
				// Filter out entities labeled as "O"
				let filtered_entities: Vec<(String, String)> =
					classification.iter().filter(|(_, label)| label != "O").cloned().collect();

				classified_sentences.push(ClassifiedSentence {
					sentence: chunk.clone(),
					entities: filtered_entities.into_iter().map(|(e, l)| (e, l, 0, 0)).collect(), // Initialize indices to 0
				});
			}

			// Convert tokenized sequences into words
			let tokenized_words = tokens_to_words(self.llm.as_ref(), &tokenized_chunks).await;

			// Match entities with tokens and get their indices
			match_entities_with_tokens(&tokenized_words, &classified_sentences)
		};

		println!("Classified Sentences: {:?}", classified_sentences);

		// Create binary pairs
		let classified_sentences_with_pairs = create_binary_pairs(&classified_sentences);
		println!("Classified Sentences with Pairs: {:?}", classified_sentences_with_pairs);

		let extended_classified_sentences_with_attention = add_attention_to_classified_sentences(
            self.llm.as_ref(),
            &classified_sentences_with_pairs,
            &tokenized_chunks,
        )
        .await?;

        println!("Classified Sentences with Attention: {:?}", extended_classified_sentences_with_attention);

        // Create the stream to yield the results
        let stream = stream! {
            for classified_sentence_with_attention in extended_classified_sentences_with_attention {
                // Create a payload
                let payload = SemanticKnowledgePayload {
                    subject: "mock123".to_string(),
                    subject_type: "mock".to_string(),
                    predicate: "mock".to_string(),
                    predicate_type: "mock".to_string(),
                    object: "mock".to_string(),
                    object_type: "mock".to_string(),
                    sentence: classified_sentence_with_attention.classified_sentence.sentence.clone(),
                    image_id: None,
                };

                let event = EventState {
                    event_type: EventType::Graph,
                    file: "mock_file".to_string(), // Update appropriately
                    doc_source: "mock_source".to_string(), // Update appropriately
                    image_id: None,
                    timestamp: 0.0,
                    payload: serde_json::to_string(&payload).unwrap_or_default(),
                };

                // Yield the created event along with the attention matrix
                yield Ok(event);
            }
        };

        Ok(Box::pin(stream))
    }
}
#[cfg(test)]
mod tests {
	use super::*;
	use common::CollectedBytes;
	use futures::StreamExt;
	use ingestors::{pdf::pdfv1::PdfIngestor, BaseIngestor};
	use llms::transformers::bert::{BertLLM, EmbedderOptions};
	use std::{fs::File, io::Read, sync::Arc};
	use tokio::sync::mpsc;
	use tokio_stream::wrappers::ReceiverStream;

	#[tokio::test]
	async fn test_txt_ingestor() {
		// // Create a sample .txt file for testing
		// let test_file_path = "/tmp/test_sample.txt";
		// let mut file = File::create(test_file_path).expect("Failed to create test file");
		// writeln!(file, "This is a test file.").expect("Failed to write to test file");

		// // Read the sample .txt file
		// let bytes = read(test_file_path.to_string()).await.expect("Failed to read test file");
		// Create a sample .pdf file for testing
		let test_file_path = "/home/nishantg/querent-main/Demo_june 6/demo_files/english_test.pdf";
		let mut file = File::open(test_file_path).expect("Failed to open test file");

		// Read the sample .pdf file into a byte vector
		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer).expect("Failed to read test file");

		// Create a CollectedBytes instance
		let collected_bytes = CollectedBytes {
			data: Some(buffer), // Use the buffer containing PDF data
			file: Some(test_file_path.into()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("pdf".to_string()),
			size: Some(10),
		};

		// Create a TxtIngestor instance
		let ingestor = PdfIngestor::new();

		// Ingest the file
		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

		// Create a tokio mpsc channel with a bounded capacity
		let (sender, receiver) = mpsc::channel(10);

		// Send the tokens to the channel
		tokio::spawn(async move {
			let mut stream = result_stream;
			while let Some(tokens) = stream.next().await {
				let tokens = tokens.unwrap();
				println!("These are the tokens in file --------------{:?}", tokens);

				// Send the IngestedTokens through the channel
				sender.send(tokens).await.unwrap();
			}
		});

		// Initialize BertLLM with EmbedderOptions
		let options = EmbedderOptions {
		    model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
		    local_dir: None,
		    revision: None,
		    distribution: None,
		};
		// let options = EmbedderOptions {
		// 	model: "/home/nishantg/querent-main/local models/geobert_files".to_string(),
		// 	local_dir: Some("/home/nishantg/querent-main/local models/geobert_files".to_string()),
		// 	revision: None,
		// 	distribution: None,
		// };
		let embedder = Arc::new(BertLLM::new(options).unwrap());

		// Create an instance of AttentionTensorsEngine
		let engine = AttentionTensorsEngine::new(embedder, vec!["joel".to_string(), "india".to_string(), "nitrogen gas".to_string()]);

		// Create an instance of Attention Tensor without fixed entities
		// let engine = AttentionTensorsEngine::new(embedder, vec![]);

		// Wrap the receiver in a tokio_stream::wrappers::ReceiverStream to convert it into a Stream
		let receiver_stream = ReceiverStream::new(receiver);

		// Pin the receiver stream to convert it to the required type
		let pinned_receiver_stream =
			Box::pin(receiver_stream) as Pin<Box<dyn Stream<Item = IngestedTokens> + Send>>;

		// Process the ingested tokens
		let mut engine_stream =
			engine.process_ingested_tokens(pinned_receiver_stream).await.unwrap();

		// Collect the results from the stream
		let mut results = Vec::new();
		while let Some(result) = engine_stream.next().await {
			results.push(result.unwrap());
		}
	}
}
