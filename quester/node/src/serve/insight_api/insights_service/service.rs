use std::{collections::HashMap, sync::Arc};

use actors::MessageBus;
use async_trait::async_trait;
use common::EventType;
use insights::{InsightError, InsightErrorKind, InsightResult};
use proto::insights::{
	InsightAnalystRequest, InsightAnalystResponse, InsightQuery, InsightQueryResponse,
	StopInsightSessionRequest, StopInsightSessionResponse,
};
use querent::InsightAgentService;
use storage::Storage;

#[async_trait]
pub trait InsightService: 'static + Send + Sync {
	/// Discover insights
	async fn provide_insight_input(
		&self,
		request: InsightQuery,
	) -> InsightResult<InsightQueryResponse>;

	/// Start Insight Session
	async fn create_insight_session(
		&self,
		request: InsightAnalystRequest,
	) -> InsightResult<InsightAnalystResponse>;

	/// Stop Insight Session
	async fn stop_insight_session(
		&self,
		request: StopInsightSessionRequest,
	) -> InsightResult<StopInsightSessionResponse>;
}

#[derive(Clone)]
pub struct InsightImpl {
	pub event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub metadata_store: Arc<dyn Storage>,
	pub insight_agent_service_message_bus: MessageBus<InsightAgentService>,
}

impl InsightImpl {
	pub fn new(
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		index_storages: Vec<Arc<dyn Storage>>,
		metadata_store: Arc<dyn Storage>,
		insight_agent_service_message_bus: MessageBus<InsightAgentService>,
	) -> Self {
		InsightImpl {
			event_storages,
			index_storages,
			insight_agent_service_message_bus,
			metadata_store,
		}
	}
}

#[async_trait]
impl InsightService for InsightImpl {
	/// Start Insight Session
	async fn create_insight_session(
		&self,
		request: InsightAnalystRequest,
	) -> InsightResult<InsightAnalystResponse> {
		let response =
			self.insight_agent_service_message_bus.ask(request.clone()).await.map_err(|e| {
				log::error!("Failed to start insight session: {}", e);
				InsightError::new(
					InsightErrorKind::Internal,
					Arc::new(anyhow::anyhow!("Failed to start insight session: {}", e)),
				)
			})?;
		match response {
			Ok(response) => {
				self.metadata_store
					.set_insight_session(&response.session_id, request.clone())
					.await
					.map_err(|e| {
						log::error!("Failed to set insight session: {}", e);
						InsightError::new(
							InsightErrorKind::Internal,
							Arc::new(anyhow::anyhow!("Failed to set insight session: {}", e)),
						)
					})?;
				Ok(response)
			},
			_ => Err(InsightError::new(
				InsightErrorKind::Internal,
				Arc::new(anyhow::anyhow!("Failed to start insight session")),
			)),
		}
	}

	/// Stop Insight Session
	async fn stop_insight_session(
		&self,
		request: StopInsightSessionRequest,
	) -> InsightResult<StopInsightSessionResponse> {
		let response = self.insight_agent_service_message_bus.ask(request).await.map_err(|e| {
			log::error!("Failed to stop insight session: {}", e);
			InsightError::new(
				InsightErrorKind::Internal,
				Arc::new(anyhow::anyhow!("Failed to stop insight session: {}", e)),
			)
		})?;
		match response {
			Ok(response) => Ok(response),
			_ => Err(InsightError::new(
				InsightErrorKind::Internal,
				Arc::new(anyhow::anyhow!("Failed to stop insight session")),
			)),
		}
	}

	/// Send Input to Insight
	async fn provide_insight_input(
		&self,
		request: InsightQuery,
	) -> InsightResult<InsightQueryResponse> {
		let response = self.insight_agent_service_message_bus.ask(request).await.map_err(|e| {
			log::error!("Failed to send input to insight: {}", e);
			InsightError::new(
				InsightErrorKind::Internal,
				Arc::new(anyhow::anyhow!("Failed to send input to insight: {}", e)),
			)
		})?;
		match response {
			Ok(response) => Ok(response),
			_ => Err(InsightError::new(
				InsightErrorKind::Internal,
				Arc::new(anyhow::anyhow!("Failed to send input to insight")),
			)),
		}
	}
}
