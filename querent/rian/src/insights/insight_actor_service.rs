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

use actors::{Actor, ActorContext, ActorExitStatus, ActorHandle, Handler, Healthz, MessageBus};
use async_trait::async_trait;
use cluster::Cluster;
use common::EventType;
use insights::{
	get_insight_info_by_id, get_insight_runner_by_id, CustomInsightOption, InsightConfig,
	InsightCustomOptionValue, InsightError, InsightErrorKind,
};
use proto::{
	InsightAnalystRequest, InsightAnalystResponse, InsightQuery, InsightQueryResponse,
	StopInsightSessionRequest, StopInsightSessionResponse,
};
use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
	sync::Arc,
};
use storage::Storage;

use crate::InsightAgent;

#[cfg(feature = "license-check")]
use crate::is_insight_allowed_by_product;

// TODO Insight Agents rethinking needed
struct InsightAgentHandles {
	mailbox: MessageBus<InsightAgent>,
	handle: ActorHandle<InsightAgent>,
}

pub struct InsightAgentService {
	node_id: String,
	cluster: Cluster,
	agent_pipelines: HashMap<String, InsightAgentHandles>,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	index_storages: Vec<Arc<dyn Storage>>,
	_license_key: Option<String>,
}

impl Debug for InsightAgentService {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("InsightAgentService")
			.field("node_id", &self.node_id)
			.field("cluster_id", &self.cluster.cluster_id())
			.finish()
	}
}

impl InsightAgentService {
	pub fn new(
		node_id: String,
		cluster: Cluster,
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		index_storages: Vec<Arc<dyn Storage>>,
		_license_key: Option<String>,
	) -> Self {
		Self {
			node_id,
			cluster,
			_license_key,
			agent_pipelines: HashMap::new(),
			event_storages,
			index_storages,
		}
	}
}

#[async_trait]
impl Actor for InsightAgentService {
	type ObservableState = ();

	fn observable_state(&self) -> Self::ObservableState {
		()
	}
}
#[async_trait]
impl Handler<InsightAnalystRequest> for InsightAgentService {
	type Reply = Result<InsightAnalystResponse, InsightError>;
	async fn handle(
		&mut self,
		request: InsightAnalystRequest,
		ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		#[cfg(feature = "license-check")]
		{
			if self._license_key.is_some() {
				let insight_id = request.id.clone();
				let license_key = self._license_key.clone().unwrap_or_default();
				let is_allowed = is_insight_allowed_by_product(license_key, insight_id)?;
				if !is_allowed {
					return Err(anyhow::anyhow!("Discovery Agent Type not allowed").into());
				}
			} else {
				return Err(anyhow::anyhow!("License Key not provided").into());
			}
		};
		let new_uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
		let current_timestamp = chrono::Utc::now().timestamp();
		let event_storages = self.event_storages.clone();
		let index_storages = self.index_storages.clone();
		if event_storages.is_empty() && index_storages.is_empty() {
			return Err(anyhow::anyhow!("No storage configurations provided").into());
		}
		let discovery_session_id = request.discovery_session_id.clone().unwrap_or_default();
		let semantic_pipeline_id = request.semantic_pipeline_id.clone().unwrap_or_default();
		let addition_options = request.additional_options.clone();
		let insight_id = request.id.clone();
		let insight = get_insight_info_by_id(&insight_id).await;
		if insight.is_none() {
			return Err(anyhow::anyhow!("Insight not found").into());
		}
		let insight = insight.unwrap();
		let mut additional_options: HashMap<String, CustomInsightOption> = HashMap::new();
		let insight_supported_options = insight.additional_options.clone();
		let keys = insight_supported_options.keys();
		for key in keys {
			if !addition_options.contains_key(key) {
				return Err(anyhow::anyhow!("Invalid Insight Option").into());
			}
			match insight_supported_options.get(key) {
				Some(option) => {
					let mut current_opt = option.clone();
					match option.clone().value {
						InsightCustomOptionValue::String { value: _, hidden } => {
							let value = addition_options.get(key).unwrap();
							current_opt.value =
								InsightCustomOptionValue::String { value: value.clone(), hidden };
							additional_options.insert(key.clone(), current_opt);
						},
						InsightCustomOptionValue::Number { min, max, step, value: _ } => {
							let value = addition_options.get(key).unwrap();
							let value: i32 = value.parse().unwrap();
							current_opt.value =
								InsightCustomOptionValue::Number { min, max, step, value };
							additional_options.insert(key.clone(), current_opt);
						},
						InsightCustomOptionValue::Boolean { value: _ } => {
							let value = addition_options.get(key).unwrap();
							let value: bool = value.parse().unwrap();
							current_opt.value = InsightCustomOptionValue::Boolean { value };
							additional_options.insert(key.clone(), current_opt);
						},
						InsightCustomOptionValue::Option { values, value: _ } => {
							let value = addition_options.get(key).unwrap();
							current_opt.value =
								InsightCustomOptionValue::Option { values, value: value.clone() };
							additional_options.insert(key.clone(), current_opt);
						},
						InsightCustomOptionValue::Button => {
							current_opt.value = InsightCustomOptionValue::Button;
							additional_options.insert(key.clone(), current_opt);
						},
					}
				},
				None => {
					return Err(anyhow::anyhow!("Invalid Insight Option").into());
				},
			}
		}
		let insight_config: InsightConfig = InsightConfig::new(
			new_uuid.clone(),
			discovery_session_id,
			semantic_pipeline_id,
			event_storages.clone(),
			index_storages.clone(),
			additional_options,
		);
		let insight = get_insight_runner_by_id(&insight_id).await;
		if insight.is_none() {
			return Err(anyhow::anyhow!("Insight not found").into());
		}
		let insight_agent = InsightAgent::new(
			insight.unwrap(),
			new_uuid.clone(),
			current_timestamp as u64,
			insight_config,
		);

		let (insight_messagebus, insight) = ctx.spawn_actor().spawn(insight_agent);
		let insight_handle = InsightAgentHandles { mailbox: insight_messagebus, handle: insight };
		self.agent_pipelines.insert(new_uuid.clone(), insight_handle);

		Ok(Ok(InsightAnalystResponse { session_id: new_uuid }))
	}
}

#[async_trait]
impl Handler<Healthz> for InsightAgentService {
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
impl Handler<StopInsightSessionRequest> for InsightAgentService {
	type Reply = Result<StopInsightSessionResponse, InsightError>;

	async fn handle(
		&mut self,
		request: StopInsightSessionRequest,
		_ctx: &ActorContext<Self>,
	) -> Result<Result<StopInsightSessionResponse, InsightError>, ActorExitStatus> {
		let agent_handle = self.agent_pipelines.remove(&request.session_id);
		if let Some(agent_handle) = agent_handle {
			let _ = agent_handle.handle.kill().await;
			return Ok(Ok(StopInsightSessionResponse { session_id: request.session_id }));
		}

		Err(anyhow::anyhow!("Insight Session not found").into())
	}
}

#[async_trait]
impl Handler<InsightQuery> for InsightAgentService {
	type Reply = Result<InsightQueryResponse, InsightError>;

	async fn handle(
		&mut self,
		request: InsightQuery,
		_ctx: &ActorContext<Self>,
	) -> Result<Result<InsightQueryResponse, InsightError>, ActorExitStatus> {
		let agent_handle = self.agent_pipelines.get(&request.session_id);
		if let Some(agent_handle) = agent_handle {
			let response = agent_handle
				.mailbox
				.ask(request)
				.await
				.map_err(|e| {
					log::error!("Failed to discover insights: {}", e);
					Err(InsightError::new(
						InsightErrorKind::Inference,
						Arc::new(anyhow::anyhow!("Failed to discover insights: {}", e)),
					))
				})
				.unwrap_or_else(|e| e);

			match response {
				Ok(response) => return Ok(Ok(response)),
				Err(e) => return Ok(Err(e)),
			}
		}

		Err(anyhow::anyhow!("Insight Session not found").into())
	}
}
