use crate::discovery_agent::DiscoveryAgent;
use actors::{Actor, ActorContext, ActorExitStatus, ActorHandle, Handler, Healthz, MessageBus};
use async_trait::async_trait;
use cluster::Cluster;
use proto::{
	DiscoveryError, DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest,
	DiscoverySessionResponse, StopDiscoverySessionRequest, StopDiscoverySessionResponse,
};
use querent_synapse::callbacks::EventType;
use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
	sync::Arc,
};
use storage::{create_storages, Storage};

struct DiscoverAgentHandle {
	mailbox: MessageBus<DiscoveryAgent>,
	handle: ActorHandle<DiscoveryAgent>,
}

pub struct DiscoveryAgentService {
	node_id: String,
	cluster: Cluster,
	agent_pipelines: HashMap<String, DiscoverAgentHandle>,
	event_storages: HashMap<EventType, Arc<dyn Storage>>,
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
		event_storages: HashMap<EventType, Arc<dyn Storage>>,
	) -> Self {
		Self { node_id, cluster, agent_pipelines: HashMap::new(), event_storages }
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
		let mut event_storages = self.event_storages.clone();

		if request.storage_configs.is_empty() && event_storages.is_empty() {
			return Err(anyhow::anyhow!("No storage configurations provided").into());
		}

		if !request.storage_configs.is_empty() {
			let (extra_events_storage, _) =
				create_storages(&request.storage_configs.clone()).await.map_err(|e| {
					log::error!("Failed to create storages: {}", e);
					e
				})?;

			event_storages.extend(extra_events_storage);
		}
		let agent = DiscoveryAgent::new(
			new_uuid.clone(),
			current_timestamp as u64,
			self.event_storages.clone(),
			request,
		);

		let (agent_messagebus, agent) = ctx.spawn_actor().spawn(agent);

		let agent_handle = DiscoverAgentHandle { mailbox: agent_messagebus, handle: agent };

		self.agent_pipelines.insert(new_uuid.clone(), agent_handle);

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
		let agent_handle = self.agent_pipelines.remove(&request.session_id);
		if let Some(agent_handle) = agent_handle {
			let _ = agent_handle.handle.kill().await;
			Ok(Ok(StopDiscoverySessionResponse { session_id: request.session_id }))
		} else {
			Err(anyhow::anyhow!("Discovery Agent not found").into())
		}
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
		let agent_handle = self.agent_pipelines.get(&request.session_id);
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
				Ok(response) => Ok(Ok(response)),
				Err(e) => Ok(Err(e)),
			}
		} else {
			Err(anyhow::anyhow!("Discovery Agent not found").into())
		}
	}
}
