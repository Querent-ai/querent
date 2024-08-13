use crate::discovery_traverser::process_auto_generated_suggestions;
use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{EventType, RuntimeType};
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
		let embedding_model = TextEmbedding::try_new(InitOptions {
			model_name: EmbeddingModel::AllMiniLML6V2,
			show_download_progress: true,
			..Default::default()
		})?;

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
		if self.embedding_model.is_none() {
			return Ok(Err(DiscoveryError::Unavailable(
				"Discovery agent embedding model is not initialized".to_string(),
			)));
		}
		//reset offset if new query
		if message.query != self.current_query {
			self.current_offset = 0;
			self.current_query = message.query.clone();
		}
		println!("This is the triple pairs ----------------------{:?}", message.top_pairs);
		let embedder = self.embedding_model.as_ref().unwrap();
		let embeddings = embedder.embed(vec![message.query.clone()], None)?;
		let current_query_embedding = embeddings[0].clone();
		let mut insights = Vec::new();
		let mut documents = Vec::new();
		let mut unique_sentences: HashSet<String> = HashSet::new();
		for (event_type, storage) in self.event_storages.iter() {
			if event_type.clone() == EventType::Vector {
				for storage in storage.iter() {
					if message.query.is_empty() {
						println!("Inside query empty--------------------------");
						let auto_suggestions = match storage.autogenerate_queries(10).await {
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
						};

						return Ok(Ok(response));
					}
					let mut fetched_results = Vec::new();
					let mut total_fetched = 0;
					while documents.len() < 10 {
						let search_results = storage
							.similarity_search_l2(
								message.session_id.clone(),
								message.query.clone(),
								self.discovery_agent_params.semantic_pipeline_id.clone(),
								&current_query_embedding.clone(),
								20,
								self.current_offset + total_fetched,
							)
							.await;
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
								for document in results.clone() {
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
														sentence: sentence.clone(),
														tags: formatted_tags,
														top_pairs: vec!["".to_string()],
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
			query: message.query.clone(),
			insights: documents.clone(),
		};

		Ok(Ok(response))
	}
}
