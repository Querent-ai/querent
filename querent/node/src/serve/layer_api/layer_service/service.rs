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

use actors::MessageBus;
use async_trait::async_trait;
use common::EventType;
use proto::layer::{
	LayerRequest, LayerResponse, LayerSessionRequest, LayerSessionRequestInfo,
	LayerSessionRequestInfoList, LayerSessionResponse, StopLayerSessionRequest,
	StopLayerSessionResponse,
};
use rian_core::layer_service::LayerAgentService;
use std::{collections::HashMap, sync::Arc};
use storage::{MetaStorage, Storage};

use crate::layer_api::layer_service::error::LayerError;

#[async_trait]
pub trait LayerService: 'static + Send + Sync {
	/// Layered insights
	async fn layer_insights(&self, request: LayerRequest) -> super::Result<LayerResponse>;

	/// Start Layer Session
	async fn start_layer_session(
		&self,
		request: LayerSessionRequest,
	) -> super::Result<LayerSessionResponse>;

	/// Stop Layer Session
	async fn stop_layer_session(
		&self,
		request: StopLayerSessionRequest,
	) -> super::Result<StopLayerSessionResponse>;

	/// Get list of all sessions
	async fn get_layer_session_list(&self) -> super::Result<LayerSessionRequestInfoList>;
}

#[derive(Clone)]
pub struct LayerImpl {
	pub event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub metadata_store: Arc<dyn MetaStorage>,
	pub layer_agent_service_message_bus: MessageBus<LayerAgentService>,
}

impl LayerImpl {
	pub fn new(
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		index_storages: Vec<Arc<dyn Storage>>,
		metadata_store: Arc<dyn MetaStorage>,
		layer_agent_service_message_bus: MessageBus<LayerAgentService>,
	) -> Self {
		LayerImpl {
			event_storages,
			index_storages,
			layer_agent_service_message_bus,
			metadata_store,
		}
	}
}

#[async_trait]
impl LayerService for LayerImpl {
	async fn layer_insights(&self, request: LayerRequest) -> super::Result<LayerResponse> {
		let response =
			self.layer_agent_service_message_bus.ask(request.clone()).await.map_err(|e| {
				log::error!("Failed to layer insights: {}", e);
				LayerError::Internal("Failed to layer insights".to_string())
			})?;
		match response {
			Ok(response) => Ok(response),
			_ => Err(LayerError::Internal("Failed to layer insights".to_string()).into()),
		}
	}

	async fn start_layer_session(
		&self,
		request: LayerSessionRequest,
	) -> super::Result<LayerSessionResponse> {
		let response =
			self.layer_agent_service_message_bus.ask(request.clone()).await.map_err(|e| {
				log::error!("Failed to start layer session: {}", e);
				LayerError::Internal("Failed to start layer session".to_string())
			})?;

		match response {
			Ok(response) => {
				self.metadata_store
					.set_layer_session(&response.session_id, request.clone())
					.await
					.map_err(|e| {
						log::error!("Failed to set insight session: {}", e);
						LayerError::Internal("Failed to set insight session".to_string())
					})?;
				Ok(response)
			},
			_ => Err(LayerError::Internal("Failed to start layer session".to_string()).into()),
		}
	}

	async fn stop_layer_session(
		&self,
		request: StopLayerSessionRequest,
	) -> super::Result<StopLayerSessionResponse> {
		let response = self.layer_agent_service_message_bus.ask(request).await.map_err(|e| {
			log::error!("Failed to stop layer session: {}", e);
			LayerError::Internal("Failed to stop layer session".to_string())
		})?;

		match response {
			Ok(response) => Ok(response),
			_ => Err(LayerError::Internal("Failed to stop layer session".to_string()).into()),
		}
	}

	async fn get_layer_session_list(&self) -> super::Result<LayerSessionRequestInfoList> {
		let metadata = self.metadata_store.get_all_layer_sessions().await.map_err(|e| {
			log::error!("Failed to get layer session list: {}", e);
			LayerError::Internal("Failed to get layer session list".to_string())
		})?;

		let mut requests: Vec<LayerSessionRequestInfo> = Vec::new();
		metadata.iter().for_each(|(session_id, session)| {
			requests.push(LayerSessionRequestInfo {
				session_id: session_id.clone(),
				request: Some(session.clone()),
			});
		});
		let response = LayerSessionRequestInfoList { requests };
		Ok(response)
	}
}
