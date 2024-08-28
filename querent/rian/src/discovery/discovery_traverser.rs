use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{get_querent_data_path, DocumentPayload, EventType, RuntimeType};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use proto::{
	discovery::{DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest},
	DiscoveryError,
};
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};
use storage::{
	utils::{extract_unique_pairs, find_intersection, get_top_k_pairs},
	QuerySuggestion, Storage,
};
use tokio::runtime::Handle;

use super::insert_discovered_knowledge_async;

pub struct DiscoveryTraverse {
	agent_id: String,
	timestamp: u64,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	discovery_agent_params: DiscoverySessionRequest,
	embedding_model: Option<TextEmbedding>,
	current_query: String,
	current_offset: i64,
	previous_query_results: String,
	previous_filtered_results: Vec<(String, String)>,
	previous_session_id: String,
	current_page_rank: i32,
}

impl DiscoveryTraverse {
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
			previous_query_results: "".to_string(),
			previous_filtered_results: Vec::new(),
			previous_session_id: "".to_string(),
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
impl Actor for DiscoveryTraverse {
	type ObservableState = ();

	async fn initialize(&mut self, _ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		let embedding_model = TextEmbedding::try_new(InitOptions {
			model_name: EmbeddingModel::AllMiniLML6V2,
			show_download_progress: true,
			cache_dir: get_querent_data_path(),
			..Default::default()
		})?;

		self.embedding_model = Some(embedding_model);
		Ok(())
	}

	fn observable_state(&self) -> Self::ObservableState {}

	fn name(&self) -> String {
		format!("DiscoveryTraverse-{}", self.agent_id)
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
impl Handler<DiscoveryRequest> for DiscoveryTraverse {
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

		if message.query != self.current_query {
			self.current_offset = 0;
			self.current_query = message.query.clone();
			self.current_page_rank = 1;
		} else {
			self.current_page_rank += 1;
		}
		let embeddings = embedder.embed(vec![&self.current_query], None)?;
		let current_query_embedding = &embeddings[0];
		let mut insights = Vec::new();
		let mut document_payloads = Vec::new();

		for (event_type, storages) in self.event_storages.iter() {
			if *event_type == EventType::Vector {
				for storage in storages.iter() {
					if message.query.is_empty() {
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
					let search_results = storage
						.similarity_search_l2(
							message.session_id.to_string(),
							message.query.to_string(),
							self.discovery_agent_params.semantic_pipeline_id.to_string(),
							current_query_embedding,
							10,
							self.current_offset,
							&vec![],
						)
						.await;
					match search_results {
						Ok(results) => {
							self.current_offset += results.len() as i64;

							let filtered_results = get_top_k_pairs(results, 3);
							let traverser_results_1 =
								storage.traverse_metadata_table(&filtered_results).await;

							if self.previous_query_results.is_empty() ||
								message.session_id != self.previous_session_id
							{
								match &traverser_results_1 {
									Ok(traverser_results) => {
										self.previous_query_results =
											match serde_json::to_string(traverser_results) {
												Ok(serialized_results) => serialized_results,
												Err(e) => {
													log::error!(
													"Failed to search for similar documents in traverser: {}",
													e
												);
													String::new()
												},
											};
										self.previous_filtered_results = filtered_results;
										self.previous_session_id = message.session_id.clone();
									},
									Err(e) => {
										log::error!("Failed to serialize traverser results: {}", e);
									},
								}

								if let Ok(traverser_results) = traverser_results_1 {
									process_documents(
										&traverser_results,
										&mut insights,
										&mut document_payloads,
										&current_query_embedding,
										&message.query,
										&message.session_id,
										&self.discovery_agent_params.semantic_pipeline_id,
									);

									tokio::spawn(insert_discovered_knowledge_async(
										storage.clone(),
										document_payloads.clone(),
									));
								} else {
									log::error!(
										"Failed to search for similar documents in traverser: {:?}",
										traverser_results_1.unwrap_err()
									);
								}
							} else {
								let previous_results: Vec<(
									String,
									String,
									String,
									String,
									String,
									String,
									String,
									f32,
								)> = match serde_json::from_str(&self.previous_query_results) {
									Ok(results) => results,
									Err(e) => {
										log::error!(
											"Failed to deserialize previous query results: {}",
											e
										);
										Vec::new()
									},
								};
								let current_results = match &traverser_results_1 {
									Ok(ref traverser_results) => traverser_results,
									Err(e) => {
										log::error!("Failed to search for similar documents in traverser: {}", e);
										&vec![]
									},
								};

								let formatted_output_1 =
									extract_unique_pairs(&current_results, &filtered_results);
								let formatted_output_2 = extract_unique_pairs(
									&previous_results,
									&self.previous_filtered_results,
								);

								let results_intersection =
									find_intersection(&formatted_output_1, &formatted_output_2);

								let final_traverser_results = if results_intersection.is_empty() {
									self.previous_filtered_results = formatted_output_1.clone();
									storage.traverse_metadata_table(&formatted_output_1).await
								} else {
									self.previous_filtered_results = results_intersection.clone();
									storage.traverse_metadata_table(&results_intersection).await
								};
								match &final_traverser_results {
									Ok(results) => {
										self.previous_query_results =
											match serde_json::to_string(results) {
												Ok(serialized_results) => serialized_results,
												Err(e) => {
													log::error!(
														"Failed to serialize traverser results: {}",
														e
													);
													String::new()
												},
											};
									},
									Err(e) => {
										log::error!("Failed to search for similar documents in traverser: {:?}", e);
									},
								}
								if let Ok(traverser_results) = final_traverser_results {
									process_documents(
										&traverser_results,
										&mut insights,
										&mut document_payloads,
										&current_query_embedding,
										&message.query,
										&message.session_id,
										&self.discovery_agent_params.semantic_pipeline_id,
									);

									tokio::spawn(insert_discovered_knowledge_async(
										storage.clone(),
										document_payloads.clone(),
									));
								} else {
									log::error!(
										"Failed to search for similar documents in traverser: {:?}",
										final_traverser_results.unwrap_err()
									);
								}
							}
						},
						Err(e) => {
							log::error!(
								"Failed to search for similar documents in traverser: {}",
								e
							);
						},
					}
				}
			}
		}

		let response = DiscoveryResponse {
			session_id: message.session_id,
			query: message.query,
			insights,
			page_ranking: self.current_page_rank,
		};

		Ok(Ok(response))
	}
}

fn process_documents(
	traverser_results: &Vec<(String, String, String, String, String, String, String, f32)>,
	insights: &mut Vec<proto::Insight>,
	document_payloads: &mut Vec<DocumentPayload>,
	current_query_embedding: &[f32],
	query: &str,
	session_id: &str,
	coll_id: &str,
) {
	for document in traverser_results {
		let tags = format!(
			"{}-{}",
			document.2.replace('_', " "), // subject
			document.3.replace('_', " "), // object
		);
		let insight = proto::Insight {
			document: document.1.to_string(),
			source: document.4.to_string(),
			relationship_strength: document.7.to_string(),
			sentence: document.5.to_string(),
			tags,
			top_pairs: vec!["".to_string()],
		};

		insights.push(insight);

		let document_payload = DocumentPayload {
			doc_id: document.1.to_string(),
			doc_source: document.4.to_string(),
			sentence: document.5.to_string(),
			knowledge: document.7.to_string(),
			subject: document.2.to_string(),
			object: document.3.to_string(),
			cosine_distance: None,
			query_embedding: Some(current_query_embedding.to_vec()),
			query: Some(query.to_string()),
			session_id: Some(session_id.to_string()),
			score: document.7,
			collection_id: coll_id.to_string(),
		};

		document_payloads.push(document_payload);
	}
}
pub fn process_auto_generated_suggestions(
	suggestions: &Vec<QuerySuggestion>,
	insights: &mut Vec<proto::Insight>,
) {
	for suggestion in suggestions {
		let unique_tags: HashSet<String> = suggestion.tags.iter().cloned().collect();
		let tags: Vec<String> = unique_tags.into_iter().collect();

		let insight = proto::Insight {
			document: suggestion.document_source.to_string(),
			source: suggestion.document_source.to_string(),
			relationship_strength: suggestion.frequency.to_string(),
			sentence: suggestion.query.to_string(),
			tags: tags.join(", "),
			top_pairs: suggestion.top_pairs.clone(),
		};

		insights.push(insight);
	}
}
