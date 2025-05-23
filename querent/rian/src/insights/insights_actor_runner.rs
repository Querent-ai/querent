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

use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::RuntimeType;
use insights::{
	Insight, InsightConfig, InsightError, InsightErrorKind, InsightInput, InsightRunner,
};
use proto::{InsightQuery, InsightQueryResponse};
use std::sync::Arc;
use tokio::runtime::Handle;

pub struct InsightAgent {
	runner: Arc<dyn InsightRunner>,
	agent_id: String,
	timestamp: u64,
}

impl InsightAgent {
	pub fn new(
		insight: Arc<dyn Insight>,
		agent_id: String,
		timestamp: u64,
		config: InsightConfig,
	) -> Self {
		let runner = insight.get_runner(&config).unwrap();
		Self { runner, agent_id, timestamp }
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
impl Actor for InsightAgent {
	type ObservableState = ();

	async fn initialize(&mut self, _ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		Ok(())
	}

	fn observable_state(&self) -> Self::ObservableState {}

	fn name(&self) -> String {
		format!("InsightsRunner-{}", self.agent_id)
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
				log::info!("Insights agent {} exiting with success", self.agent_id);
			},
		}
		Ok(())
	}
}

#[async_trait]
impl Handler<InsightQuery> for InsightAgent {
	type Reply = Result<InsightQueryResponse, InsightError>;

	async fn handle(
		&mut self,
		message: InsightQuery,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let runner = self.runner.clone();
		let agent_id = self.agent_id.clone();
		// Create a JSON object with session_id and query
		let data_to_send =
			serde_json::json!({ "session_id": message.session_id, "query": message.query });

		// Directly use the JSON object as InsightInput
		let insight_input = InsightInput { data: data_to_send };
		let response = tokio::spawn(async move {
			let insight_output = runner.run(insight_input).await;
			match insight_output {
				Ok(output) => Ok(InsightQueryResponse {
					session_id: agent_id,
					response: output.data.to_string(),
					query: message.query,
				}),
				Err(e) => Err(e),
			}
		})
		.await;
		match response {
			Ok(response) => Ok(response),
			Err(e) => Ok(Err(InsightError::new(
				InsightErrorKind::Inference,
				Arc::new(anyhow::anyhow!("Error running insight: {:?}", e)),
			))),
		}
	}
}
