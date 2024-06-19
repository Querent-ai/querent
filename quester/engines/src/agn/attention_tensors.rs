use crate::utils::{
	add_attention_to_classified_sentences, create_binary_pairs, label_entities_in_sentences, match_entities_with_tokens, merge_similar_relations, remove_newlines, split_into_chunks, tokens_to_words, ClassifiedSentence, ClassifiedSentenceWithRelations
};
use async_stream::stream;
use async_trait::async_trait;
use common::{EventState, EventType, SemanticKnowledgePayload};
use futures::{Stream, StreamExt};
use llms::llm::LLM;
use proto::semantics::IngestedTokens;
use std::{pin::Pin, sync::Arc};

use crate::{Engine, EngineError, EngineResult};
use crate::agn::attention_based_filter::{IndividualFilter,Token, SearchBeam};
use crate::agn::attention_based_search::{Entity,EntityPair, perform_search};


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
		// Match entities with tokens and get their indices
		let classified_sentences = if !self.entities.is_empty() {
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


		// Create binary pairs
		let classified_sentences_with_pairs = create_binary_pairs(&classified_sentences);

		let extended_classified_sentences_with_attention = add_attention_to_classified_sentences(
            self.llm.as_ref(),
            &classified_sentences_with_pairs,
            &tokenized_chunks,
        )
        .await?;

        // Perform search and filter on classified sentences with attention
        // List to hold all sentences with their relations
        let mut all_sentences_with_relations = Vec::new();

        for (classified, tokenized_chunk) in extended_classified_sentences_with_attention.iter().zip(tokenized_chunks.iter()) {
            let attention_matrix = classified.attention_matrix.as_ref().unwrap();
            let pairs = &classified.classified_sentence.pairs;

			// Convert token IDs to words
            let token_words = self.llm.tokens_to_words(tokenized_chunk).await;

            // Create Token objects with text and lemma being the same (assuming no lemma info)
            let tokens: Vec<Token> = token_words.iter().map(|word| {
                Token {
                    text: word.clone(),
                    lemma: word.clone(),
                }
            }).collect();

            let filter = IndividualFilter::new(tokens, true, 0.05); // Adjust parameters as needed

			let mut sentence_relations = Vec::new();

			for (head_text, head_start, head_end, tail_text, tail_start, tail_end) in pairs {
                let head = Entity {
                    name: head_text.clone(),
                    start_idx: *head_start,
                    end_idx: *head_end,
                };
                let tail = Entity {
                    name: tail_text.clone(),
                    start_idx: *tail_start,
                    end_idx: *tail_end,
                };

                let entity_pair = EntityPair {
                    head_entity: head.clone(),
                    tail_entity: tail.clone(),
                    context: classified.classified_sentence.sentence.clone(),
                };

                let search_results = perform_search(
                    head.start_idx,
                    attention_matrix,
                    &entity_pair,
                    5, // Number of search candidates to keep
                    true, // Require contiguous relations
                    5, // Maximum relation length
                ).unwrap_or_else(|e| {
                    println!("Error during search: {:?}", e);
                    Vec::new()
                });

                let search_beams: Vec<SearchBeam> = search_results.into_iter()
                    .map(|sr| SearchBeam {
                        rel_tokens: sr.relation_tokens,
                        score: sr.total_score,
                    }).collect();

                let head_tail_relations = filter.filter(search_beams, &head, &tail);
                sentence_relations.push(head_tail_relations);
            }

			all_sentences_with_relations.push(ClassifiedSentenceWithRelations {
                classified_sentence: classified.classified_sentence.clone(),
                // attention_matrix: attention_matrix.clone(),
                relations: sentence_relations,
            });
        }

		println!("All Sentences with Relations: {:?}", all_sentences_with_relations);
		// Merge similar relations
        merge_similar_relations(&mut all_sentences_with_relations);
		
        println!("All Sentences with Relations: {:?}", all_sentences_with_relations);

        let stream = stream! {
			for sentence_with_relations in all_sentences_with_relations {
				println!("Printing sentence_with_relations.relations----- ---{:?}", &sentence_with_relations.relations);
				
				for head_tail_relation in &sentence_with_relations.relations {
					for (predicate, score) in &head_tail_relation.relations {
						// Create a payload
						let payload = SemanticKnowledgePayload {
							subject: head_tail_relation.head.name.clone(),
							subject_type: "unlabelled".to_string(), // Placeholder for future implementation
							predicate: predicate.clone(),
							predicate_type: "relation".to_string(), // Assuming the type is "relation"
							object: head_tail_relation.tail.name.clone(),
							object_type: "unlabelled".to_string(), // Placeholder for future implementation
							sentence: sentence_with_relations.classified_sentence.sentence.clone(),
							image_id: None,
							// score: *score, // Include the predicate score
						};
		
						let event = EventState {
							event_type: EventType::Graph,
							file: "mock_file".to_string(), // Update appropriately
							doc_source: "mock_source".to_string(), // Update appropriately
							image_id: None,
							timestamp: 0.0,
							payload: serde_json::to_string(&payload).unwrap_or_default(),
						};
		
						// Yield the created event
						yield Ok(event);
					}
				}
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
		let test_file_path = "/home/nishantg/querent-main/Demo_june 6/demo_files/small.pdf";
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
		let engine = AttentionTensorsEngine::new(embedder, vec!["oil".to_string(), "gas".to_string(),"porosity".to_string(), "joel".to_string(), "india".to_string(),"microsoft".to_string(), "nitrogen gas".to_string()]);

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
