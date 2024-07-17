use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{DocumentPayload, EventType, RuntimeType};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use proto::{
	discovery::{DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest},
	DiscoveryError,
};
use std::{collections::HashMap, sync::Arc};
use storage::{
	utils::{extract_unique_pairs, find_intersection, get_top_k_pairs},
	Storage,
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
		if self.embedding_model.is_none() {
			return Ok(Err(DiscoveryError::Unavailable(
				"Discovery agent embedding model is not initialized".to_string(),
			)));
		}
		if message.query.is_empty() {
			return Ok(Err(DiscoveryError::Internal("Discovery agent query is empty".to_string())));
		}
		// Reset offset if new query
		if message.query != self.current_query {
			self.current_offset = 0;
			self.current_query = message.query.clone();
		}
		let embedder = self.embedding_model.as_ref().unwrap();
		let embeddings = embedder.embed(vec![message.query.clone()], None)?;
		let current_query_embedding = embeddings[0].clone();
		let mut insights = Vec::new();
		let mut document_payloads = Vec::new();

		for (event_type, storages) in self.event_storages.iter() {
			if event_type.clone() == EventType::Vector {
				for storage in storages.iter() {
					let search_results = storage
						.similarity_search_l2(
							message.session_id.clone(),
							message.query.clone(),
							self.discovery_agent_params.semantic_pipeline_id.clone(),
							&current_query_embedding.clone(),
							10,
							self.current_offset,
						)
						.await;
					match search_results {
						Ok(results) => {
							self.current_offset += results.len() as i64;

							// Filter results
							let filtered_results = get_top_k_pairs(results.clone(), 2);
							let traverser_results_1 =
								storage.traverse_metadata_table(filtered_results.clone()).await;

							// Check and populate previous_query_results if empty
							if self.previous_query_results.is_empty() ||
								message.session_id != self.previous_session_id
							{
								match &traverser_results_1 {
									Ok(ref traverser_results) => {
										self.previous_query_results =
											serde_json::to_string(traverser_results)
												.unwrap_or_default();
										self.previous_filtered_results = filtered_results.clone();
										self.previous_session_id = message.session_id.clone();
									},
									Err(e) => {
										log::error!("Failed to serialize traverser results: {}", e);
									},
								}

								// Format documents and insert discovered knowledge
								if let Ok(ref traverser_results) = traverser_results_1 {
									process_documents(
										traverser_results,
										&mut insights,
										&mut document_payloads,
										current_query_embedding.clone(),
										message.query.clone(),
										message.session_id.clone(),
										self.discovery_agent_params.semantic_pipeline_id.clone(),
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
								// If previous_query_results is not empty, parse and combine with current results
								let previous_results: Vec<(
									i32,
									String,
									String,
									String,
									String,
									String,
									String,
									f32,
								)> = serde_json::from_str(&self.previous_query_results)
									.unwrap_or_default();
								let current_results = match &traverser_results_1 {
									Ok(ref traverser_results) => traverser_results.clone(),
									Err(e) => {
										log::error!("Failed to search for similar documents in traverser: {}", e);
										vec![]
									},
								};

								let formatted_output_1 = extract_unique_pairs(
									current_results.clone(),
									filtered_results.clone(),
								);
								let formatted_output_2 = extract_unique_pairs(
									previous_results.clone(),
									self.previous_filtered_results.clone(),
								);

								// Run intersection function
								let results_intersection = find_intersection(
									formatted_output_1.clone(),
									formatted_output_2.clone(),
								);

								let final_traverser_results = if results_intersection.is_empty() {
									self.previous_filtered_results = formatted_output_1.clone();
									storage
										.traverse_metadata_table(formatted_output_1.clone())
										.await
								} else {
									self.previous_filtered_results = results_intersection.clone();
									storage
										.traverse_metadata_table(results_intersection.clone())
										.await
								};
								match final_traverser_results.clone() {
									Ok(ref results) => {
										self.previous_query_results =
											serde_json::to_string(results).unwrap_or_default();
									},
									Err(e) => {
										log::error!("Failed to search for similar documents in traverser: {:?}", e);
									},
								}

								// Format documents and insert discovered knowledge for final results
								if let Ok(ref traverser_results) = final_traverser_results {
									process_documents(
										traverser_results,
										&mut insights,
										&mut document_payloads,
										current_query_embedding.clone(),
										message.query.clone(),
										message.session_id.clone(),
										self.discovery_agent_params.semantic_pipeline_id.clone(),
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
			query: message.query.clone(),
			insights,
		};

		Ok(Ok(response))
	}
}

fn process_documents(
	traverser_results: &Vec<(i32, String, String, String, String, String, String, f32)>,
	insights: &mut Vec<proto::Insight>,
	document_payloads: &mut Vec<DocumentPayload>,
	current_query_embedding: Vec<f32>,
	query: String,
	session_id: String,
	coll_id: String,
) {
	for document in traverser_results {
		let tags = format!(
			"{}, {}",
			document.2.replace('_', " "), // subject
			document.3.replace('_', " "), // object
		);
		let insight = proto::Insight {
			document: document.1.to_string(),              // doc_id
			source: document.4.clone(),                    // doc_source
			relationship_strength: document.7.to_string(), // knowledge (score)
			sentence: document.5.clone(),                  // sentence
			tags,
		};

		insights.push(insight);

		let document_payload = DocumentPayload {
			doc_id: document.1.to_string(),
			doc_source: document.4.clone(),
			sentence: document.5.clone(),
			knowledge: document.7.to_string(),
			subject: document.2.clone(),
			object: document.3.clone(),
			cosine_distance: None, // Assign appropriately if available
			query_embedding: Some(current_query_embedding.clone()),
			query: Some(query.clone()),
			session_id: Some(session_id.clone()),
			score: document.7, // Use the correct field for score if different
			collection_id: coll_id.clone(),
		};

		document_payloads.push(document_payload);
	}
}
