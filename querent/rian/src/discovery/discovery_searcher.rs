use crate::discovery_traverser::process_auto_generated_suggestions;
use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{get_querent_data_path, EventType, RuntimeType};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use proto::{
	discovery::{DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest},
	DiscoveryError,
};
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};
use storage::Storage;
use tokio::runtime::Handle;

use super::insert_discovered_knowledge_async;

pub struct DiscoverySearch {
	agent_id: String,
	timestamp: u64,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	discovery_agent_params: DiscoverySessionRequest,
	embedding_model: Option<TextEmbedding>,
	current_query: String,
	current_offset: i64,
	current_top_pairs: Vec<String>,
	current_page_rank: i32,
}

impl DiscoverySearch {
	pub fn new(
		agent_id: String,
		timestamp: u64,
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		discovery_agent_params: DiscoverySessionRequest,
	) -> Self {
		Self {
			agent_id,
			timestamp,
			event_storages,
			discovery_agent_params,
			embedding_model: None,
			current_query: "".to_string(),
			current_offset: 0,
			current_top_pairs: vec![],
			current_page_rank: 0,
		}
	}

	pub fn get_timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn set_timestamp(&mut self, timestamp: u64) {
		self.timestamp = timestamp;
	}

	pub fn get_agent_id(&self) -> String {
		self.agent_id.clone()
	}
}

#[async_trait]
impl Actor for DiscoverySearch {
	type ObservableState = ();

	async fn initialize(&mut self, _ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		let model_details = InitOptions::new(EmbeddingModel::AllMiniLML6V2)
			.with_cache_dir(get_querent_data_path())
			.with_show_download_progress(true);
		let embedding_model = TextEmbedding::try_new(model_details)?;

		self.embedding_model = Some(embedding_model);
		Ok(())
	}

	fn observable_state(&self) -> Self::ObservableState {}

	fn name(&self) -> String {
		format!("DiscoverySearch-{}", self.agent_id)
	}

	fn queue_capacity(&self) -> QueueCapacity {
		QueueCapacity::Unbounded
	}

	fn runtime_handle(&self) -> Handle {
		RuntimeType::NonBlocking.get_runtime_handle()
	}

	#[inline]
	fn yield_after_each_message(&self) -> bool {
		false
	}

	async fn finalize(
		&mut self,
		exit_status: &ActorExitStatus,
		_ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		match exit_status {
			ActorExitStatus::DownstreamClosed |
			ActorExitStatus::Killed |
			ActorExitStatus::Failure(_) |
			ActorExitStatus::Panicked => return Ok(()),
			ActorExitStatus::Quit | ActorExitStatus::Success => {
				log::info!("Discovery agent {} exiting with success", self.agent_id);
			},
		}
		Ok(())
	}
}

#[async_trait]
impl Handler<DiscoveryRequest> for DiscoverySearch {
	type Reply = Result<DiscoveryResponse, DiscoveryError>;

	async fn handle(
		&mut self,
		message: DiscoveryRequest,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let embedder = match self.embedding_model.as_ref() {
			Some(embedder) => embedder,
			None => {
				return Ok(Err(DiscoveryError::Unavailable(
					"Discovery agent embedding model is not initialized".to_string(),
				)));
			},
		};
		let message_set: HashSet<_> = message.top_pairs.iter().collect();
		let current_set: HashSet<_> = self.current_top_pairs.iter().collect();
		if message.query != self.current_query || message_set != current_set {
			self.current_offset = 0;
			self.current_page_rank = 1;
			self.current_query = message.query.clone();
			self.current_top_pairs = message.top_pairs.clone();
		} else {
			self.current_page_rank += 1;
		}
		let embeddings = embedder.embed(vec![&self.current_query], None)?;
		let current_query_embedding = &embeddings[0];
		let mut insights = Vec::new();
		let mut documents = Vec::new();
		let mut unique_sentences: HashSet<String> = HashSet::new();
		let mut top_pairs_incoming = message.top_pairs.clone();
		let subjects_objects: Vec<String> = message
			.top_pairs
			.iter()
			.flat_map(|pair| pair.split(" - ").map(String::from))
			.collect();
		let mut top_pair_embeddings: Vec<Vec<f32>> = Vec::new();
		let query_weight = 2.0;
		let entity_weight = 0.5;
		for chunk in subjects_objects.chunks(2) {
			if chunk.len() == 2 {
				let subject_embedding = embedder.embed(vec![&chunk[0]], None)?;
				let object_embedding = embedder.embed(vec![&chunk[1]], None)?;
				let combined_embedding: Vec<f32> = current_query_embedding
					.iter()
					.zip(subject_embedding[0].iter())
					.zip(object_embedding[0].iter())
					.map(|((q, s), o)| query_weight * q + entity_weight * (s + o))
					.collect();

				top_pair_embeddings.push(combined_embedding);
			}
		}
		for (event_type, storage) in self.event_storages.iter() {
			if *event_type == EventType::Vector {
				for storage in storage.iter() {
					if self.current_query.is_empty() && self.current_top_pairs.is_empty() {
						let auto_suggestions = match storage.autogenerate_queries(3).await {
							Ok(suggestions) => suggestions,
							Err(e) => {
								tracing::info!("Failed to auto-generate queries: {:?}", e);
								vec![]
							},
						};

						process_auto_generated_suggestions(&auto_suggestions, &mut insights);

						let response = DiscoveryResponse {
							session_id: message.session_id,
							query: "Auto-generated suggestions".to_string(),
							insights,
							page_ranking: self.current_page_rank,
						};

						return Ok(Ok(response));
					}
					let mut fetched_results = Vec::new();
					let mut total_fetched = 0;

					if top_pairs_incoming.len() < 10 {
						let auto_top_pairs_suggestions = match storage.autogenerate_queries(1).await
						{
							Ok(suggestions) => suggestions,
							Err(e) => {
								tracing::info!("Failed to generate top pairs suggestions: {:?}", e);
								vec![]
							},
						};
						let mut additional_pairs = Vec::new();
						let mut seen_pairs = std::collections::HashSet::new();

						for pair in &top_pairs_incoming {
							seen_pairs.insert(pair.clone());
						}

						for suggestion in auto_top_pairs_suggestions {
							for pair in suggestion.top_pairs {
								if seen_pairs.insert(pair.clone()) {
									additional_pairs.push(pair);
								}
							}
						}
						let needed_items = 10 - top_pairs_incoming.len();
						for pair in additional_pairs.into_iter().take(needed_items) {
							top_pairs_incoming.push(pair);
						}
					}

					while documents.len() < 10 {
						let search_results;
						if message.query.is_empty() && !message.top_pairs.is_empty() {
							search_results = storage
								.filter_and_query(
									&message.session_id.clone(),
									&message.top_pairs,
									10,
									self.current_offset + total_fetched,
								)
								.await;
						} else {
							search_results = storage
								.similarity_search_l2(
									message.session_id.clone(),
									message.query.clone(),
									self.discovery_agent_params.semantic_pipeline_id.clone(),
									current_query_embedding,
									10,
									self.current_offset + total_fetched,
									&top_pair_embeddings,
									Some("".to_string()),
								)
								.await;
						}
						match search_results {
							Ok(results) => {
								if results.is_empty() {
									break;
								}
								total_fetched += results.len() as i64;
								fetched_results.extend(results.clone());
								let mut combined_results: HashMap<String, (HashSet<String>, f32)> =
									HashMap::new();
								let mut ordered_sentences: Vec<String> = Vec::new();
								for document in &results {
									let tag = format!(
										"{}-{}",
										document.subject.replace('_', " "),
										document.object.replace('_', " "),
									);
									if let Some((existing_tags, total_strength)) =
										combined_results.get_mut(&document.sentence)
									{
										existing_tags.insert(tag);
										*total_strength += document.score;
									} else {
										let mut tags_set = HashSet::new();
										tags_set.insert(tag);
										combined_results.insert(
											document.sentence.clone(),
											(tags_set, document.score),
										);
										ordered_sentences.push(document.sentence.clone());
									}
								}
								for sentence in ordered_sentences {
									if documents.len() >= 10 {
										let index = match results
											.iter()
											.position(|doc| doc.sentence == sentence)
										{
											Some(idx) => idx,
											None => {
												tracing::error!(
													"Unable to find sentence in results: {}",
													sentence
												);
												continue;
											},
										};
										total_fetched -= results.len() as i64 - index as i64;
										break;
									}
									if unique_sentences.insert(sentence.clone()) {
										if let Some((tags_set, total_strength)) =
											combined_results.get(&sentence)
										{
											let formatted_tags = tags_set
												.clone()
												.into_iter()
												.collect::<Vec<_>>()
												.join(", ");
											if let Some(source) =
												results.iter().find(|doc| doc.sentence == sentence)
											{
												let formatted_document =
													proto::discovery::Insight {
														document: source.doc_id.clone(),
														source: source.doc_source.clone(),
														relationship_strength: total_strength
															.to_string(),
														sentence,
														tags: formatted_tags,
														top_pairs: top_pairs_incoming.to_vec(),
													};

												documents.push(formatted_document);
											} else {
												tracing::error!("Unable to find source document for sentence in Retriever: {}", sentence);
											}
										} else {
											tracing::error!(
												"Unable to process insights in Retriever: {}",
												sentence
											);
										}
									} else {
										break;
									}
								}
							},
							Err(e) => {
								log::error!("Failed to search for similar documents: {}", e);
								break;
							},
						}
					}

					self.current_offset += total_fetched;
					tokio::spawn(insert_discovered_knowledge_async(
						storage.clone(),
						fetched_results,
					));
				}
			}
		}
		let response = DiscoveryResponse {
			session_id: message.session_id,
			query: self.current_query.to_string(),
			insights: documents,
			page_ranking: self.current_page_rank,
		};

		Ok(Ok(response))
	}
}
