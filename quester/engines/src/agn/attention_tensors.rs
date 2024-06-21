use crate::utils::{
	add_attention_to_classified_sentences, calculate_biased_sentence_embedding,
	create_binary_pairs, extract_entities_and_types, label_entities_in_sentences,
	match_entities_with_tokens, merge_similar_relations, remove_newlines, split_into_chunks,
	tokens_to_words, ClassifiedSentence, ClassifiedSentenceWithRelations,
};
use async_stream::stream;
use async_trait::async_trait;
use common::{EventState, EventType, SemanticKnowledgePayload, VectorPayload};
use fastembed::TextEmbedding;
use futures::{Stream, StreamExt};
use llms::llm::LLM;
use proto::semantics::IngestedTokens;
use std::{pin::Pin, sync::Arc};

use crate::{
	agn::{
		attention_based_filter::{IndividualFilter, SearchBeam, Token},
		attention_based_search::{perform_search, Entity, EntityPair},
	},
	Engine, EngineError, EngineErrorKind, EngineResult,
};

pub struct AttentionTensorsEngine {
	pub llm: Arc<dyn LLM>,
	pub entities: Vec<String>,
	pub sample_entities: Vec<String>,
	embedding_model: Option<TextEmbedding>,
	ner_llm: Option<Arc<dyn LLM>>,
}

impl AttentionTensorsEngine {
	pub fn new(
		llm: Arc<dyn LLM>,
		entities: Vec<String>,
		sample_entities: Vec<String>,
		embedding_model: Option<TextEmbedding>,
		ner_llm: Option<Arc<dyn LLM>>, // Accept as optional
	) -> Self {
		Self { llm, entities, sample_entities, embedding_model, ner_llm }
	}
}

#[async_trait]
impl Engine for AttentionTensorsEngine {
	async fn process_ingested_tokens<'life0>(
		&'life0 self,
		token_stream: Pin<Box<dyn Stream<Item = IngestedTokens> + Send + 'life0>>,
	) -> EngineResult<Pin<Box<dyn Stream<Item = EngineResult<EventState>> + Send + 'life0>>> {
		// Await the maximum tokens value outside the stream! block
		let max_tokens = self.llm.maximum_tokens().await;
		// Make copies of necessary parts of `self`
		let mut entities = self.entities.clone();
		let mut sample_entities = self.sample_entities.clone();
		let llm = self.llm.clone();

		if self.embedding_model.is_none() {
			return Err(EngineError::new(
				EngineErrorKind::ModelError,
				anyhow::anyhow!("Embedding model not initialized").into(),
			));
		}

		let embedder = self.embedding_model.as_ref().unwrap();
		// Process the token stream to get chunks
		let stream = stream! {
			let mut token_stream = token_stream;
			while let Some(token) = token_stream.next().await {
				let doc_source = token.doc_source.clone(); // Assuming doc_source is of type Option<String>
				let file = token.file.clone(); // Assuming file is of type Option<String>
				let cleaned_data: Vec<String> =
					token.data.into_iter().map(|s| remove_newlines(&s)).collect();
				// let doc_source: = token.doc_source;

				let mut all_chunks = Vec::new();
				for data in cleaned_data {
					let split_chunks = split_into_chunks(max_tokens, &data);
					all_chunks.extend(split_chunks);
				}

				let mut tokenized_chunks = Vec::new();
				for chunk in &all_chunks {
					let tokenized_chunk =
						llm.tokenize(chunk).await.map_err(|e| EngineError::from(e))?;
					tokenized_chunks.push(tokenized_chunk);
				}
				let classified_sentences = if !entities.is_empty() {
					let initial_classified_sentences =
						label_entities_in_sentences(&entities, &all_chunks);
					let tokenized_words = tokens_to_words(llm.as_ref(), &tokenized_chunks).await;
					println!("Classificatiuon Results ---------{:?}", initial_classified_sentences);
					match_entities_with_tokens(&tokenized_words, &initial_classified_sentences)
				} else {
					let mut model_inputs = Vec::new();
					let mut tokenized_chunks_ner = Vec::new();
					let tokenized_words = tokens_to_words(llm.as_ref(), &tokenized_chunks).await;
					let mut classified_sentences = Vec::new();
					if let Some(ref ner_llm) = self.ner_llm {
						for chunk in &all_chunks {
							let tokenized_chunk = ner_llm.tokenize(chunk).await.map_err(|e| EngineError::from(e))?;
							tokenized_chunks_ner.push(tokenized_chunk);
						}
						for tokens in &tokenized_chunks_ner {
							let model_input = ner_llm.model_input(tokens.clone()).await.map_err(|e| EngineError::from(e))?;
							model_inputs.push(model_input);
						}
						println!("Labell--------");
						let mut classification_results = Vec::new();
						for input in &model_inputs {
							let classification_result = ner_llm.token_classification(input.clone(), None).await.map_err(|e| EngineError::from(e))?;
							classification_results.push(classification_result);
						}


						for (chunk, classification) in all_chunks.iter().zip(classification_results.iter()) {
							let filtered_entities: Vec<(String, String)> = classification.iter().filter(|(_, label)| label != "O").cloned().collect();

							classified_sentences.push(ClassifiedSentence {
								sentence: chunk.clone(),
								entities: filtered_entities.into_iter().map(|(e, l)| (e, l, 0, 0)).collect(),
							});
						}
						println!("Classification Results ---------{:?}", classified_sentences);
					}

					match_entities_with_tokens(&tokenized_words, &classified_sentences)
				};
				println!("Classificatiuon Results 222222222---------{:?}", classified_sentences);
				let classified_sentences_with_pairs = create_binary_pairs(&classified_sentences);

				let extended_classified_sentences_with_attention = add_attention_to_classified_sentences(
					llm.as_ref(),
					&classified_sentences_with_pairs,
					&tokenized_chunks,
				)
				.await?;

				let mut all_sentences_with_relations = Vec::new();

				for (classified, tokenized_chunk) in extended_classified_sentences_with_attention.iter().zip(tokenized_chunks.iter()) {
					let attention_matrix = classified.attention_matrix.as_ref().unwrap().clone();
					let pairs = &classified.classified_sentence.pairs;

					let token_words = llm.tokens_to_words(tokenized_chunk).await;

					let tokens: Vec<Token> = token_words.iter().map(|word| {
						Token {
							text: word.clone(),
							lemma: word.clone(),
						}
					}).collect();

					let filter = IndividualFilter::new(tokens, true, 0.05);

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
							&attention_matrix,
							&entity_pair,
							5,
							true,
							5,
						).unwrap_or_else(|_e| {
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
						attention_matrix: Some(attention_matrix),
						relations: sentence_relations,
					});
				}

				merge_similar_relations(&mut all_sentences_with_relations);

				if !all_sentences_with_relations.is_empty()  && entities.is_empty(){
					println!("Entities are ------{:?}", entities);
					println!("Sample Entities are ------{:?}", sample_entities);
					(entities, sample_entities) = extract_entities_and_types(all_sentences_with_relations.clone());
				}
				println!("Entities are ------{:?}", entities);
				println!("Sample Entities are ------{:?}", sample_entities);
				for sentence_with_relations in all_sentences_with_relations {
					for head_tail_relation in &sentence_with_relations.relations {
						for (predicate, _score) in &head_tail_relation.relations {
							// Find the index of the head and tail entities
							let head_index = entities.iter().position(|e| e == &head_tail_relation.head.name);
							let tail_index = entities.iter().position(|e| e == &head_tail_relation.tail.name);

							// Assign the types based on the indices
							let subject_type = head_index.and_then(|i| sample_entities.get(i)).unwrap_or(&"unlabelled".to_string()).clone();
							let object_type = tail_index.and_then(|i| sample_entities.get(i)).unwrap_or(&"unlabelled".to_string()).clone();

							let payload = SemanticKnowledgePayload {
								subject: head_tail_relation.head.name.clone().to_string(),
								subject_type: subject_type.to_string(),
								predicate: predicate.clone().to_string(),
								predicate_type: "relation".to_string(),
								object: head_tail_relation.tail.name.clone().to_string(),
								object_type: object_type.to_string(),
								sentence: sentence_with_relations.classified_sentence.sentence.clone().to_string(),
								image_id: None,
							};
							let event = EventState {
								event_type: EventType::Graph,
								file: file.to_string(),
								doc_source: doc_source.to_string(),
								image_id: None,
								timestamp: 0.0,
								payload: serde_json::to_string(&payload).unwrap_or_default(),
							};
							yield Ok(event);
						}
					}
					let attention_matrix = sentence_with_relations.attention_matrix.as_ref().unwrap();

					for relation in &sentence_with_relations.relations {
						let head_entity = &relation.head.name;
						let tail_entity = &relation.tail.name;

						// Assuming the indices are available in the Entity struct
						let head_start_index = relation.head.start_idx;
						let head_end_index = relation.head.end_idx;
						let tail_start_index = relation.tail.start_idx;
						let tail_end_index = relation.tail.end_idx;
						for (predicate, score) in &relation.relations {
							let biased_embedding = calculate_biased_sentence_embedding(
								embedder,
								attention_matrix,
								head_entity,
								tail_entity,
								predicate,
								score,
								&sentence_with_relations.classified_sentence.sentence,
								head_start_index,
								head_end_index,
								tail_start_index,
								tail_end_index,
							).await.map_err(|e| EngineError::new(EngineErrorKind::ModelError, Arc::new(e.into())))?;
							let id = format!("{}-{}-{}", head_entity, predicate, tail_entity);
							let payload = VectorPayload {
								id,
								embeddings: biased_embedding.clone(),
								size: biased_embedding.len() as u64,
								namespace: predicate.to_string(),
								sentence: Some(sentence_with_relations.classified_sentence.sentence.clone()),
								document_source: Some("mock".to_string()),
								blob: Some("mock".to_string()),
							};
							let event = EventState {
								event_type: EventType::Vector,
								file: file.to_string(),
								doc_source: doc_source.to_string(),
								image_id: None,
								timestamp: 0.0,
								payload: serde_json::to_string(&payload).unwrap_or_default(),
							};
							yield Ok(event);
						}
					}
				}
			}
		};

		Ok(Box::pin(stream))
	}
}
// #[cfg(test)]
// mod tests {
// 	use super::*;
// 	use common::CollectedBytes;
// 	use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
// 	use futures::StreamExt;
// 	use ingestors::{pdf::pdfv1::PdfIngestor, BaseIngestor};
// 	use llms::transformers::bert::{BertLLM, EmbedderOptions};
// 	use std::{fs::File, io::Read, sync::Arc};
// 	use tokio::sync::mpsc;
// 	use tokio_stream::wrappers::ReceiverStream;

// 	#[tokio::test]
// 	async fn test_txt_ingestor() {
// 		let test_file_path = "/home/nishantg/querent-main/Demo_june 6/demo_files/test.pdf";
// 		let mut file = File::open(test_file_path).expect("Failed to open test file");

// 		// Read the sample .pdf file into a byte vector
// 		let mut buffer = Vec::new();
// 		file.read_to_end(&mut buffer).expect("Failed to read test file");

// 		// Create a CollectedBytes instance
// 		let collected_bytes = CollectedBytes {
// 			data: Some(buffer), // Use the buffer containing PDF data
// 			file: Some(test_file_path.into()),
// 			doc_source: Some("test_source".to_string()),
// 			eof: false,
// 			extension: Some("pdf".to_string()),
// 			size: Some(10),
// 		};

// 		// Create a TxtIngestor instance
// 		let ingestor = PdfIngestor::new();

// 		// Ingest the file
// 		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

// 		// Create a tokio mpsc channel with a bounded capacity
// 		let (sender, receiver) = mpsc::channel(10);

// 		// Send the tokens to the channel
// 		tokio::spawn(async move {
// 			let mut stream = result_stream;
// 			while let Some(tokens) = stream.next().await {
// 				let tokens = tokens.unwrap();

// 				// Send the IngestedTokens through the channel
// 				sender.send(tokens).await.unwrap();
// 			}
// 		});

// 		// let entities = vec![
// 		// 	"oil".to_string(),
// 		// 	"gas".to_string(),
// 		// 	"porosity".to_string(),
// 		// 	"joel".to_string(),
// 		// 	"india".to_string(),
// 		// 	"microsoft".to_string(),
// 		// 	"nitrogen gas".to_string(),
// 		// 	"deposition".to_string(),
// 		// ];
// 		let entities = vec![];

// 		// Initialize NER model only if fixed_entities is not defined or empty
// 		let ner_llm: Option<Arc<dyn LLM>> = if entities.is_empty() {
// 			let ner_options = EmbedderOptions {
// 				model: "/home/nishantg/querent-main/local models/geobert_files".to_string(),
// 				local_dir : Some("/home/nishantg/querent-main/local models/geobert_files".to_string()),
// 				revision: None,
// 				distribution: None,
// 			};

// 			Some(Arc::new(BertLLM::new(ner_options).unwrap()) as Arc<dyn LLM>)
// 		} else {
// 			None  // Some(Arc::new(DummyLLM) as Arc<dyn LLM>)
// 		};

// 		// Initialize BertLLM with EmbedderOptions
// 		let options = EmbedderOptions {
// 			model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
// 			local_dir: None,
// 			revision: None,
// 			distribution: None,
// 		};
// 		// let options = EmbedderOptions {
// 		// 	model: "/home/nishantg/querent-main/local models/geobert_files".to_string(),
// 		// 	local_dir: Some("/home/nishantg/querent-main/local models/geobert_files".to_string()),
// 		// 	revision: None,
// 		// 	distribution: None,
// 		// };
// 		let embedder = Arc::new(BertLLM::new(options).unwrap());

// 		// Initialize the embedding model
// 		let embedding_model = TextEmbedding::try_new(InitOptions {
// 			model_name: EmbeddingModel::AllMiniLML6V2,
// 			show_download_progress: true,
// 			..Default::default()
// 		})
// 		.unwrap();

// 		// Create an instance of AttentionTensorsEngine
// 		let engine = AttentionTensorsEngine::new(
// 			embedder,
// 			entities,
// 			vec![
// 				"oil".to_string(),
// 				"gas".to_string(),
// 				"porosity".to_string(),
// 				"joel".to_string(),
// 				"india".to_string(),
// 				"microsoft".to_string(),
// 				"nitrogen gas".to_string(),
// 			],
// 			Some(embedding_model),
// 			ner_llm,
// 		);

// 		// Create an instance of Attention Tensor without fixed entities
// 		// let engine = AttentionTensorsEngine::new(embedder, vec![]);

// 		// Wrap the receiver in a tokio_stream::wrappers::ReceiverStream to convert it into a Stream
// 		let receiver_stream = ReceiverStream::new(receiver);

// 		// Pin the receiver stream to convert it to the required type
// 		let pinned_receiver_stream =
// 			Box::pin(receiver_stream) as Pin<Box<dyn Stream<Item = IngestedTokens> + Send>>;

// 		// Process the ingested tokens
// 		let mut engine_stream =
// 			engine.process_ingested_tokens(pinned_receiver_stream).await.unwrap();

// 		// Collect the results from the stream
// 		let mut results = Vec::new();
// 		while let Some(result) = engine_stream.next().await {
// 			results.push(result.unwrap());
// 		}
// 	}
// }
