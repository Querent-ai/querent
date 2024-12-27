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

use crate::utils::{
	add_attention_to_classified_sentences, calculate_biased_sentence_embedding,
	create_binary_pairs, extract_entities_and_types, generate_custom_comb_uuid,
	label_entities_in_sentences, match_entities_with_tokens, merge_similar_relations,
	remove_newlines, select_highest_score_relation, split_into_chunks, tokens_to_words,
	ClassifiedSentence, ClassifiedSentenceWithRelations,
};
use async_stream::stream;
use async_trait::async_trait;
use common::{EventState, EventType, SemanticKnowledgePayload, VectorPayload};
use fastembed::TextEmbedding;
use futures::Stream;
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
use tokio::sync::mpsc::Receiver;

/// Engine implementation for processing tokens using attention tensors and LLMs.
pub struct AttentionTensorsEngine {
	/// The main large language model (LLM) used by the engine.
	pub llm: Arc<dyn LLM>,
	/// List of entity names.
	pub entities: Vec<String>,
	/// Sample entity names for comparison and classification.
	pub sample_entities: Vec<String>,
	/// Optional text embedding model.
	embedding_model: Option<TextEmbedding>,
	/// Optional Named Entity Recognition (NER) model.
	ner_llm: Option<Arc<dyn LLM>>,
}

impl AttentionTensorsEngine {
	/// Creates a new `AttentionTensorsEngine` with the specified components.
	///
	/// # Arguments
	///
	/// * `llm` - The main large language model.
	/// * `entities` - List of entity names.
	/// * `sample_entities` - Sample entity names for comparison and classification.
	/// * `embedding_model` - Optional text embedding model.
	/// * `ner_llm` - Optional Named Entity Recognition (NER) model.
	///
	/// # Returns
	///
	/// A new instance of `AttentionTensorsEngine`.
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
	/// Processes ingested tokens and generates events based on attention tensors and entity relations.
	///
	/// # Arguments
	///
	/// * `token_stream` - A stream of ingested tokens.
	///
	/// # Returns
	///
	/// A result containing a stream of events or an error.
	async fn process_ingested_tokens<'life0>(
		&'life0 self,
		token_stream: Receiver<IngestedTokens>,
	) -> EngineResult<Pin<Box<dyn Stream<Item = EngineResult<EventState>> + Send + 'life0>>> {
		let embedder = if let Some(embedder) = self.embedding_model.as_ref() {
			embedder
		} else {
			return Err(EngineError::new(
				EngineErrorKind::ModelError,
				anyhow::anyhow!("AGN Embedding model not initialized").into(),
			));
		};
		let default_label = "unlabelled".to_string();
		let stream = stream! {
			let max_tokens = self.llm.maximum_tokens().await;
			let mut entities = self.entities.clone();
			let mut sample_entities = self.sample_entities.clone();
			let llm = &self.llm;
			let mut token_stream = token_stream;
			while let Some(token) = token_stream.recv().await {
				if token.data.is_empty() {
					continue;
				}
				let doc_source = &token.doc_source;
				let file = &token.file;
				let cleaned_data: Vec<String> =
					token.data.into_iter().map(|s| remove_newlines(&s)).collect();
				let source_id = &token.source_id;
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
							let model_input = ner_llm.model_input(tokens.to_vec()).await.map_err(|e| EngineError::from(e))?;
							model_inputs.push(model_input);
						}
						let mut classification_results = Vec::new();
						for input in &model_inputs {
							let classification_result = ner_llm.token_classification(input.clone(), None).await.map_err(|e| EngineError::from(e))?;
							classification_results.push(classification_result);
						}
						for (chunk, classification) in all_chunks.iter().zip(classification_results.iter()) {
							let filtered_entities: Vec<(String, String)> = classification.iter().filter(|(_, label)| label != "O").cloned().collect();
							classified_sentences.push(ClassifiedSentence {
								sentence: chunk.to_string(),
								entities: filtered_entities
											.into_iter()
											.map(|(e, l)| (e.to_lowercase(), l, 0, 0))
											.collect(),
									});
						}
					}
					match_entities_with_tokens(&tokenized_words, &classified_sentences)
				};
				let classified_sentences_with_pairs = create_binary_pairs(&classified_sentences);
				let extended_classified_sentences_with_attention = add_attention_to_classified_sentences(
					llm.as_ref(),
					&classified_sentences_with_pairs,
					&tokenized_chunks,
				)
				.await?;
				let mut all_sentences_with_relations = Vec::new();
				for (classified, tokenized_chunk) in extended_classified_sentences_with_attention.iter().zip(tokenized_chunks.iter()) {
					let attention_matrix = match classified.attention_matrix.as_ref() {
						Some(matrix) => matrix.clone(),
						None => {
							log::error!("Unable to process Attention matrix in AGN");
							return;
						},
					};
					let pairs = &classified.classified_sentence.pairs;
					let token_words = llm.tokens_to_words(tokenized_chunk).await;
					let tokens: Vec<Token> = token_words.iter().map(|word| {
						Token {
							text: word.to_string(),
							lemma: word.to_string(),
						}
					}).collect();
					let filter = IndividualFilter::new(tokens, true, 0.05);
					let mut sentence_relations = Vec::new();
					for (head_text, head_start, head_end, tail_text, tail_start, tail_end) in pairs {
						let head = Entity {
							name: head_text.to_string(),
							start_idx: *head_start,
							end_idx: *head_end,
						};
						let tail = Entity {
							name: tail_text.to_string(),
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
						let highest_scored_relation = select_highest_score_relation(&head_tail_relations);
						sentence_relations.push(highest_scored_relation);
					}
					all_sentences_with_relations.push(ClassifiedSentenceWithRelations {
						classified_sentence: classified.classified_sentence.clone(),
						attention_matrix: Some(attention_matrix),
						relations: sentence_relations,
					});
				}
				merge_similar_relations(&mut all_sentences_with_relations);
				if !all_sentences_with_relations.is_empty()  && entities.is_empty(){
					(entities, sample_entities) = extract_entities_and_types(all_sentences_with_relations.clone());
				}
				for sentence_with_relations in all_sentences_with_relations {
					let mut event_ids = Vec::new();
					for head_tail_relation in &sentence_with_relations.relations {
						for (predicate, _score) in &head_tail_relation.relations {
							let event_id = generate_custom_comb_uuid();
							event_ids.push(event_id.clone());
							let head_index = entities.iter().position(|e| e == &head_tail_relation.head.name);
							let tail_index = entities.iter().position(|e| e == &head_tail_relation.tail.name);
							let subject_type = head_index
								.and_then(|i| sample_entities.get(i))
								.unwrap_or(&default_label);
							let object_type = tail_index
								.and_then(|i| sample_entities.get(i))
								.unwrap_or(&default_label);
							let payload = SemanticKnowledgePayload {
								subject: head_tail_relation.head.name.to_string(),
								subject_type: subject_type.to_string(),
								predicate: predicate.to_string(),
								predicate_type: "relation".to_string(),
								object: head_tail_relation.tail.name.to_string(),
								object_type: object_type.to_string(),
								sentence: sentence_with_relations.classified_sentence.sentence.to_string(),
								image_id: token.image_id.clone(),
								blob: Some("mock".to_string()),
								source_id: source_id.to_string(),
								event_id: event_id,
							};
							let serialized_payload = match serde_json::to_string(&payload) {
								Ok(json) => json,
								Err(e) => {
									log::error!("Failed to serialize payload: {:?}", e);
									String::new()
								},
							};
							let event = EventState {
								event_type: EventType::Graph,
								file: file.to_string(),
								doc_source: doc_source.to_string(),
								image_id: token.image_id.clone(),
								timestamp: 0.0,
								payload: serialized_payload,
							};
							yield Ok(event);
						}
					}
					let attention_matrix = match sentence_with_relations.attention_matrix.as_ref() {
						Some(matrix) => matrix,
						None => {
							log::error!("Unable to process Attention matrix in AGN while creating Vector Event Payload.");
							return;
						}
					};
					let mut i = 0;
					for relation in &sentence_with_relations.relations {
						let head_entity = &relation.head.name;
						let tail_entity = &relation.tail.name;
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
							let payload = VectorPayload {
								event_id: event_ids[i].to_string(),
								embeddings: biased_embedding,
								score: *score as f32,
							};
							let serialized_payload = match serde_json::to_string(&payload) {
								Ok(json) => json,
								Err(e) => {
									log::error!("Failed to serialize payload: {:?}", e);
									String::new()
								},
							};
							let event = EventState {
								event_type: EventType::Vector,
								file: file.to_string(),
								doc_source: doc_source.to_string(),
								image_id: token.image_id.clone(),
								timestamp: 0.0,
								payload: serialized_payload,
							};
							i += 1;
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
// 	use llms::transformers::{bert::{BertLLM, EmbedderOptions}, roberta::roberta::RobertaLLM};
// use proto::semantics::OneDriveConfig;
// 	use std::{fs::File, io::Read, sync::Arc};
// 	use tokio::sync::mpsc;
// 	use tokio_stream::wrappers::ReceiverStream;
// 	use sources::onedrive::onedrive::OneDriveSource;
// 	use sources::Source;

// 	#[tokio::test]
// 	async fn test_txt_ingestor() {
// 		let test_file_path = "/home/nishantg/querent-main/Demo_june 6/demo_files/english_test.pdf";
// 		let mut file = File::open(test_file_path).expect("Failed to open test file");
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
// 			source_id: "Abcd".to_string(),
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
// 				// model: "/home/nishantg/querent-main/local models/geobert_files".to_string(),
// 				// local_dir : Some("/home/nishantg/querent-main/local models/geobert_files".to_string()),
// 				model: "Davlan/xlm-roberta-base-wikiann-ner".to_string(),
// 				// model: "deepset/roberta-base-squad2".to_string(),
// 				local_dir : None,
// 				revision: None,
// 				distribution: None,
// 			};

// 			// Some(Arc::new(BertLLM::new(ner_options).unwrap()) as Arc<dyn LLM>)
// 			Some(Arc::new(RobertaLLM::new(ner_options).unwrap()) as Arc<dyn LLM>)
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

// 	#[tokio::test]
//     async fn test_pdf_ingestor() {

//         let onedrive_config = OneDriveConfig {
// 			client_id: "c7c05424-b4d5-4af9-8f97-9e2de234b1b4".to_string(),
//             client_secret: "I-08Q~fZ~Vsbm6Mc7rj4sqyzgjlYIA5WN5jG.cLn".to_string(),
//             redirect_uri: "http://localhost:8000/callback".to_string(),
//             refresh_token: "M.C540_BAY.0.U.-Cg3wuI8L3FPX!LmwIHH1W8ChFNgervWiVAwuppNW9EC1W8iXHE797KeL!OU6*ywNfZD1*FVuVNroTPyH3HrzaP3ZiG!xepBUpmDKq1NjmXDFya6rlBABG*ahheNyOHv*WV9gYb*voX11ic00XJmxYyzEnHCxjbZ5SU75rWqzAgltIilcVoQm8VhLSeMYpRkUzDWS*Jeg6Ht8AuPJHpmetwdME7b33pOiKupGlFKn7OH1SoO7Xsc6JYcp96hneg8TS8mLg1!tVN9NkRcv1q1JjxxgLPPRXn*Xub7Y61rew91E9GdaXTAzJzFiRAL8ISH2*vq4gEzxmAG*wtfV9nMzT85JH2xxpdMvrvaXsrMrqJUm".to_string(),
//             folder_path: "/testing".to_string(),
// 			id: "test".to_string(),
// 		};

// 		let drive_storage = OneDriveSource::new(onedrive_config).await.unwrap();
// 		// Create a TxtIngestor instance
//         let ingestor = PdfIngestor::new();
// 		let connectivity = drive_storage.check_connectivity().await;

// 		println!("Connectivity: {:?}", connectivity);

// 		let result = drive_storage.poll_data().await;

// 		let mut stream = result.unwrap();
// 		while let Some(item) = stream.next().await {
// 			match item {
// 				Ok(collected_bytes) => {
// 					println!("Here  ");
// 					let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();
// 					println!("Got the stream: ");
// 					let mut stream = result_stream;
// 					while let Some(tokens) = stream.next().await {
// 						let tokens = tokens.unwrap();
// 						println!("These are the tokens in file --------------{:?}", tokens);
// 					}
// 				},
// 				Err(err) => eprintln!("Expected successful data collection {:?}", err),
// 			}
// 		}

// 	}
// }
