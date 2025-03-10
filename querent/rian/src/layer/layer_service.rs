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

#[cfg(feature = "license-check")]
use crate::is_layer_agent_type_allowed;
use crate::layer_link_prediction::LayerLink;
use actors::{Actor, ActorContext, ActorExitStatus, ActorHandle, Handler, Healthz, MessageBus};
use async_trait::async_trait;
use cluster::Cluster;
use common::EventType;
use proto::layer::{
	LayerAgentType, LayerError, LayerRequest, LayerResponse, LayerSessionRequest,
	LayerSessionResponse, StopLayerSessionRequest, StopLayerSessionResponse,
};
use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
	sync::Arc,
};
use storage::Storage;

/// Linker via layer
struct LayerLinkHandle {
	mailbox: MessageBus<LayerLink>,
	handle: ActorHandle<LayerLink>,
}

/// Layer Agent Service
pub struct LayerAgentService {
	node_id: String,
	cluster: Cluster,
	searcher_pipelines: HashMap<String, LayerLinkHandle>,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	_license_key: Option<String>,
}

impl Debug for LayerAgentService {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("LayerAgentService")
			.field("node_id", &self.node_id)
			.field("cluster_id", &self.cluster.cluster_id())
			.finish()
	}
}

impl LayerAgentService {
	pub fn new(
		node_id: String,
		cluster: Cluster,
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		_license_key: Option<String>,
	) -> Self {
		Self { node_id, cluster, searcher_pipelines: HashMap::new(), event_storages, _license_key }
	}
}

#[async_trait]
impl Actor for LayerAgentService {
	type ObservableState = ();

	fn observable_state(&self) -> Self::ObservableState {
		()
	}
}
#[async_trait]
impl Handler<LayerSessionRequest> for LayerAgentService {
	type Reply = Result<LayerSessionResponse, LayerError>;
	async fn handle(
		&mut self,
		request: LayerSessionRequest,
		ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let new_uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
		let current_timestamp = chrono::Utc::now().timestamp();
		let event_storages = self.event_storages.clone();
		#[cfg(feature = "license-check")]
		{
			if self._license_key.is_some() {
				let license_key = self._license_key.clone().unwrap_or_default();
				let is_allowed = is_layer_agent_type_allowed(
					license_key,
					&request.session_type.clone().unwrap_or(LayerAgentType::Link),
				)?;
				if !is_allowed {
					return Err(anyhow::anyhow!("Layer Agent Type not allowed").into());
				}
			} else {
				return Err(anyhow::anyhow!("License Key not provided").into());
			}
		};
		match request.session_type.clone().unwrap_or(LayerAgentType::Link) {
			LayerAgentType::Link => {
				let search = LayerLink::new(
					new_uuid.clone(),
					current_timestamp as u64,
					event_storages.clone(),
					request.clone(),
				);

				let (search_messagebus, search) = ctx.spawn_actor().spawn(search);
				let search_handle = LayerLinkHandle { mailbox: search_messagebus, handle: search };
				self.searcher_pipelines.insert(new_uuid.clone(), search_handle);
			},
		}

		Ok(Ok(LayerSessionResponse { session_id: new_uuid }))
	}
}

#[async_trait]
impl Handler<Healthz> for LayerAgentService {
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
impl Handler<StopLayerSessionRequest> for LayerAgentService {
	type Reply = Result<StopLayerSessionResponse, LayerError>;

	async fn handle(
		&mut self,
		request: StopLayerSessionRequest,
		_ctx: &ActorContext<Self>,
	) -> Result<Result<StopLayerSessionResponse, LayerError>, ActorExitStatus> {
		let search_handle = self.searcher_pipelines.remove(&request.session_id);
		if let Some(search_handle) = search_handle {
			let _ = search_handle.handle.kill().await;
			return Ok(Ok(StopLayerSessionResponse { session_id: request.session_id }));
		}

		Err(anyhow::anyhow!("Layer Session not found").into())
	}
}

#[async_trait]
impl Handler<LayerRequest> for LayerAgentService {
	type Reply = Result<LayerResponse, LayerError>;

	async fn handle(
		&mut self,
		request: LayerRequest,
		_ctx: &ActorContext<Self>,
	) -> Result<Result<LayerResponse, LayerError>, ActorExitStatus> {
		let search_handle = self.searcher_pipelines.get(&request.session_id);
		if let Some(search_handle) = search_handle {
			let response = search_handle
				.mailbox
				.ask(request)
				.await
				.map_err(|e| {
					log::error!("Failed to discover insights: {}", e);
					Err(LayerError::Internal("Failed to discover insights".to_string()))
				})
				.unwrap_or_else(|e| e);
			match response {
				Ok(response) => return Ok(Ok(response)),
				Err(e) => return Ok(Err(e)),
			}
		}

		Err(anyhow::anyhow!("Layer Session not found").into())
	}
}
