use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::RuntimeType;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use proto::{
	discovery::{DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest},
	DiscoveryError,
};
use querent_synapse::callbacks::EventType;
use std::{collections::HashMap, sync::Arc};
use storage::Storage;
use tokio::runtime::Handle;

pub struct DiscoveryTraverser {
	agent_id: String,
	timestamp: u64,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	discovery_agent_params: DiscoverySessionRequest,
	embedding_model: Option<TextEmbedding>,
}

impl DiscoveryTraverser {
	pub fn new(
		agent_id: String,
		timestamp: u64,
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		discovery_agent_params: DiscoverySessionRequest,
	) -> Self {
		Self { agent_id, timestamp, event_storages, discovery_agent_params, embedding_model: None }
	}
}

#[async_trait]
impl Actor for DiscoveryTraverser {
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
		format!("DiscoveryTraverser-{}", self.agent_id)
	}

	fn queue_capacity(&self) -> QueueCapacity {
		QueueCapacity::Bounded(self.discovery_agent_params.max_message_memory_size as usize)
	}

	fn runtime_handle(&self) -> Handle {
		RuntimeType::NonBlocking.get_runtime_handle()
	}

	#[inline]
	fn yield_after_each_message(&self) -> bool {
		false
	}
}

#[async_trait]
impl Handler<DiscoveryRequest> for DiscoveryTraverser {
	type Reply = Result<DiscoveryResponse, DiscoveryError>;

	async fn handle(
		&mut self,
		message: DiscoveryRequest,
		ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		// First search in the vector database
		if self.embedding_model.is_none() {
			return Ok(Err(DiscoveryError::Unavailable(
				"Discovery agent embedding model is not initialized".to_string(),
			)));
		}
		println!("---------------------------------INSIDEEEEEEEEEEEEEEE");
		let embedder = self.embedding_model.as_ref().unwrap();
		let embeddings = embedder.embed(vec![message.query.clone()], None)?;
		let current_query_embedding = embeddings[0].clone();
		let mut documents = Vec::new();
		for (event_type, storage) in self.event_storages.iter() {
			if event_type.clone() == EventType::Graph {
				for storage in storage.iter() {
					let search_results = storage
						.similarity_search_l2(
							self.discovery_agent_params.semantic_pipeline_id.clone(),
							&current_query_embedding.clone(),
							10,
						)
						.await;
					match search_results {
						Ok(results) => {
							documents.extend(results);
						},
						Err(e) => {
							log::error!("Failed to search for similar documents: {}", e);
						},
					}
				}
			}
		}

		// Second search function goes here
		// Example:
		// let second_search_results = second_search_function(&documents);

		Ok(Ok(DiscoveryResponse {
			session_id: message.session_id,
			query: message.query.clone(),
			insights: documents
				.iter()
				.map(|doc| proto::discovery::Insight {
					title: format!("{} - {}", doc.doc_source, doc.doc_id),
					description: serde_json::to_string(doc).unwrap(),
				})
				.collect(),
		}))
	}
}
