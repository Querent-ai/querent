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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use actors::MessageBus;
use async_trait::async_trait;
use common::EventType;
use proto::{
	DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest, DiscoverySessionRequestInfo,
	DiscoverySessionRequestInfoList, DiscoverySessionResponse, StopDiscoverySessionRequest,
	StopDiscoverySessionResponse,
};
use rian_core::discovery_service::DiscoveryAgentService;
use std::{collections::HashMap, sync::Arc};
use storage::{MetaStorage, Storage};

use crate::discovery_api::discovery_service::error::DiscoveryError;

#[async_trait]
pub trait DiscoveryService: 'static + Send + Sync {
	/// Discover insights
	async fn discover_insights(
		&self,
		request: DiscoveryRequest,
	) -> super::Result<DiscoveryResponse>;

	/// Start Discovery Session
	async fn start_discovery_session(
		&self,
		request: DiscoverySessionRequest,
	) -> super::Result<DiscoverySessionResponse>;

	/// Stop Discovery Session
	async fn stop_discovery_session(
		&self,
		request: StopDiscoverySessionRequest,
	) -> super::Result<StopDiscoverySessionResponse>;

	/// Get list of all sessions
	async fn get_discovery_session_list(&self) -> super::Result<DiscoverySessionRequestInfoList>;
}

#[derive(Clone)]
pub struct DiscoveryImpl {
	pub event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub metadata_store: Arc<dyn MetaStorage>,
	pub discovery_agent_service_message_bus: MessageBus<DiscoveryAgentService>,
}

impl DiscoveryImpl {
	pub fn new(
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		index_storages: Vec<Arc<dyn Storage>>,
		metadata_store: Arc<dyn MetaStorage>,
		discovery_agent_service_message_bus: MessageBus<DiscoveryAgentService>,
	) -> Self {
		DiscoveryImpl {
			event_storages,
			index_storages,
			discovery_agent_service_message_bus,
			metadata_store,
		}
	}
}

#[async_trait]
impl DiscoveryService for DiscoveryImpl {
	async fn discover_insights(
		&self,
		request: DiscoveryRequest,
	) -> super::Result<DiscoveryResponse> {
		let response = self
			.discovery_agent_service_message_bus
			.ask(request.clone())
			.await
			.map_err(|e| {
				log::error!("Failed to discover insights: {}", e);
				DiscoveryError::Internal("Failed to discover insights".to_string())
			})?;
		match response {
			Ok(response) => Ok(response),
			_ => Err(DiscoveryError::Internal("Failed to discover insights".to_string()).into()),
		}
	}

	async fn start_discovery_session(
		&self,
		request: DiscoverySessionRequest,
	) -> super::Result<DiscoverySessionResponse> {
		let response = self
			.discovery_agent_service_message_bus
			.ask(request.clone())
			.await
			.map_err(|e| {
				log::error!("Failed to start discovery session: {}", e);
				DiscoveryError::Internal("Failed to start discovery session".to_string())
			})?;

		match response {
			Ok(response) => {
				self.metadata_store
					.set_discovery_session(&response.session_id, request.clone())
					.await
					.map_err(|e| {
						log::error!("Failed to set insight session: {}", e);
						DiscoveryError::Internal("Failed to set insight session".to_string())
					})?;
				Ok(response)
			},
			_ =>
				Err(DiscoveryError::Internal("Failed to start discovery session".to_string())
					.into()),
		}
	}

	async fn stop_discovery_session(
		&self,
		request: StopDiscoverySessionRequest,
	) -> super::Result<StopDiscoverySessionResponse> {
		let response =
			self.discovery_agent_service_message_bus.ask(request).await.map_err(|e| {
				log::error!("Failed to stop discovery session: {}", e);
				DiscoveryError::Internal("Failed to stop discovery session".to_string())
			})?;

		match response {
			Ok(response) => Ok(response),
			_ =>
				Err(DiscoveryError::Internal("Failed to stop discovery session".to_string()).into()),
		}
	}

	async fn get_discovery_session_list(&self) -> super::Result<DiscoverySessionRequestInfoList> {
		let metadata = self.metadata_store.get_all_discovery_sessions().await.map_err(|e| {
			log::error!("Failed to get discovery session list: {}", e);
			DiscoveryError::Internal("Failed to get discovery session list".to_string())
		})?;

		let mut requests: Vec<DiscoverySessionRequestInfo> = Vec::new();
		metadata.iter().for_each(|(session_id, session)| {
			requests.push(DiscoverySessionRequestInfo {
				session_id: session_id.clone(),
				request: Some(session.clone()),
			});
		});
		let response = DiscoverySessionRequestInfoList { requests };
		Ok(response)
	}
}
