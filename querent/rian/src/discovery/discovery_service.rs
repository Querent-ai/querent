#[cfg(feature = "license-check")]
use crate::is_discovery_agent_type_allowed;
use crate::{discovery_searcher::DiscoverySearch, discovery_traverser::DiscoveryTraverse};
use actors::{Actor, ActorContext, ActorExitStatus, ActorHandle, Handler, Healthz, MessageBus};
use async_trait::async_trait;
use cluster::Cluster;
use common::EventType;
use proto::{
	DiscoveryAgentType, DiscoveryError, DiscoveryRequest, DiscoveryResponse,
	DiscoverySessionRequest, DiscoverySessionResponse, StopDiscoverySessionRequest,
	StopDiscoverySessionResponse,
};
use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
	sync::Arc,
};
use storage::Storage;

/// Traverser via discovery
struct DiscoveryTraverseHandle {
	mailbox: MessageBus<DiscoveryTraverse>,
	handle: ActorHandle<DiscoveryTraverse>,
}

/// Searcher via discovery
struct DiscoverSearchHandle {
	mailbox: MessageBus<DiscoverySearch>,
	handle: ActorHandle<DiscoverySearch>,
}

/// Discovery Agent Service
pub struct DiscoveryAgentService {
	node_id: String,
	cluster: Cluster,
	traverse_pipelines: HashMap<String, DiscoveryTraverseHandle>,
	searcher_pipelines: HashMap<String, DiscoverSearchHandle>,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	_license_key: Option<String>,
}

impl Debug for DiscoveryAgentService {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DiscoveryAgentService")
			.field("node_id", &self.node_id)
			.field("cluster_id", &self.cluster.cluster_id())
			.finish()
	}
}

impl DiscoveryAgentService {
	pub fn new(
		node_id: String,
		cluster: Cluster,
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		_license_key: Option<String>,
	) -> Self {
		Self {
			node_id,
			cluster,
			traverse_pipelines: HashMap::new(),
			searcher_pipelines: HashMap::new(),
			event_storages,
			_license_key,
		}
	}
}

#[async_trait]
impl Actor for DiscoveryAgentService {
	type ObservableState = ();

	fn observable_state(&self) -> Self::ObservableState {
		()
	}
}
#[async_trait]
impl Handler<DiscoverySessionRequest> for DiscoveryAgentService {
	type Reply = Result<DiscoverySessionResponse, DiscoveryError>;
	async fn handle(
		&mut self,
		request: DiscoverySessionRequest,
		ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let new_uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
		let current_timestamp = chrono::Utc::now().timestamp();
		let event_storages = self.event_storages.clone();
		#[cfg(feature = "license-check")]
		{
			if self._license_key.is_some() {
				let license_key = self._license_key.clone().unwrap_or_default();
				let is_allowed = is_discovery_agent_type_allowed(
					license_key,
					&request.session_type.clone().unwrap_or(DiscoveryAgentType::Retriever),
				)?;
				if !is_allowed {
					return Err(anyhow::anyhow!("Discovery Agent Type not allowed").into());
				}
			} else {
				return Err(anyhow::anyhow!("License Key not provided").into());
			}
		};
		match request.session_type.clone().unwrap_or(DiscoveryAgentType::Retriever) {
			DiscoveryAgentType::Retriever => {
				let search = DiscoverySearch::new(
					new_uuid.clone(),
					current_timestamp as u64,
					event_storages.clone(),
					request.clone(),
				);

				let (search_messagebus, search) = ctx.spawn_actor().spawn(search);
				let search_handle =
					DiscoverSearchHandle { mailbox: search_messagebus, handle: search };
				self.searcher_pipelines.insert(new_uuid.clone(), search_handle);
			},
			DiscoveryAgentType::Traverser => {
				let search = DiscoveryTraverse::new(
					new_uuid.clone(),
					current_timestamp as u64,
					event_storages.clone(),
					request.clone(),
				);

				let (traverse_messagebus, traverse) = ctx.spawn_actor().spawn(search);
				let traverse_handle =
					DiscoveryTraverseHandle { mailbox: traverse_messagebus, handle: traverse };
				self.traverse_pipelines.insert(new_uuid.clone(), traverse_handle);
			},
		}

		Ok(Ok(DiscoverySessionResponse { session_id: new_uuid }))
	}
}

#[async_trait]
impl Handler<Healthz> for DiscoveryAgentService {
	type Reply = bool;

	async fn handle(
		&mut self,
		_msg: Healthz,
		_ctx: &ActorContext<Self>,
	) -> Result<bool, ActorExitStatus> {
		Ok(true)
	}
}

#[async_trait]
impl Handler<StopDiscoverySessionRequest> for DiscoveryAgentService {
	type Reply = Result<StopDiscoverySessionResponse, DiscoveryError>;

	async fn handle(
		&mut self,
		request: StopDiscoverySessionRequest,
		_ctx: &ActorContext<Self>,
	) -> Result<Result<StopDiscoverySessionResponse, DiscoveryError>, ActorExitStatus> {
		let agent_handle = self.traverse_pipelines.remove(&request.session_id);
		if let Some(agent_handle) = agent_handle {
			let _ = agent_handle.handle.kill().await;
			return Ok(Ok(StopDiscoverySessionResponse { session_id: request.session_id }));
		}

		let search_handle = self.searcher_pipelines.remove(&request.session_id);
		if let Some(search_handle) = search_handle {
			let _ = search_handle.handle.kill().await;
			return Ok(Ok(StopDiscoverySessionResponse { session_id: request.session_id }));
		}

		Err(anyhow::anyhow!("Discovery Session not found").into())
	}
}

#[async_trait]
impl Handler<DiscoveryRequest> for DiscoveryAgentService {
	type Reply = Result<DiscoveryResponse, DiscoveryError>;

	async fn handle(
		&mut self,
		request: DiscoveryRequest,
		_ctx: &ActorContext<Self>,
	) -> Result<Result<DiscoveryResponse, DiscoveryError>, ActorExitStatus> {
		let agent_handle = self.traverse_pipelines.get(&request.session_id);
		if let Some(agent_handle) = agent_handle {
			let response = agent_handle
				.mailbox
				.ask(request)
				.await
				.map_err(|e| {
					log::error!("Failed to discover insights: {}", e);
					Err(DiscoveryError::Internal("Failed to discover insights".to_string()))
				})
				.unwrap_or_else(|e| e);

			match response {
				Ok(response) => return Ok(Ok(response)),
				Err(e) => return Ok(Err(e)),
			}
		}

		let search_handle = self.searcher_pipelines.get(&request.session_id);
		if let Some(search_handle) = search_handle {
			let response = search_handle
				.mailbox
				.ask(request)
				.await
				.map_err(|e| {
					log::error!("Failed to discover insights: {}", e);
					Err(DiscoveryError::Internal("Failed to discover insights".to_string()))
				})
				.unwrap_or_else(|e| e);

			match response {
				Ok(response) => return Ok(Ok(response)),
				Err(e) => return Ok(Err(e)),
			}
		}

		Err(anyhow::anyhow!("Discovery Session not found").into())
	}
}
