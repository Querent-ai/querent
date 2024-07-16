use actors::MessageBus;
use async_trait::async_trait;
use common::EventType;
use proto::{
	DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest, DiscoverySessionRequestInfo,
	DiscoverySessionRequestInfoList, DiscoverySessionResponse, StopDiscoverySessionRequest,
	StopDiscoverySessionResponse,
};
use rian::discovery_service::DiscoveryAgentService;
use std::{collections::HashMap, sync::Arc};
use storage::Storage;

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
	pub metadata_store: Arc<dyn Storage>,
	pub discovery_agent_service_message_bus: MessageBus<DiscoveryAgentService>,
}

impl DiscoveryImpl {
	pub fn new(
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		index_storages: Vec<Arc<dyn Storage>>,
		metadata_store: Arc<dyn Storage>,
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
