use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{EventType, RuntimeType};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use proto::{
	discovery::{DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest},
	DiscoveryError,
};
use std::{collections::HashMap, sync::Arc};
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
		if message.query.is_empty() {
			return Ok(Err(DiscoveryError::Internal("Discovery agent query is empty".to_string())));
		}
		//reset offset if new query
		if message.query != self.current_query {
			self.current_offset = 0;
			self.current_query = message.query.clone();
		}
		let embedder = self.embedding_model.as_ref().unwrap();
		let embeddings = embedder.embed(vec![message.query.clone()], None)?;
		let current_query_embedding = embeddings[0].clone();
		let mut documents = Vec::new();
		for (event_type, storage) in self.event_storages.iter() {
			if event_type.clone() == EventType::Vector {
				for storage in storage.iter() {
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
							let res = results.clone();
							for document in results {
								let tags = format!(
									"{}-{}",
									document.subject.replace('_', " "),
									document.object.replace('_', " "),
								);
								let formatted_document = proto::discovery::Insight {
									document: document.doc_id,
									source: document.doc_source,
									relationship_strength: document.score.to_string(),
									sentence: document.sentence,
									tags,
								};

								documents.push(formatted_document);
							}

							tokio::spawn(insert_discovered_knowledge_async(storage.clone(), res));
						},
						Err(e) => {
							log::error!("Failed to search for similar documents: {}", e);
						},
					}
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
