use std::{collections::HashMap, sync::Arc};

use crate::IndexingStatistics;
use actors::{Actor, ActorContext, ActorExitStatus, Handler};
use async_trait::async_trait;
use common::TerimateSignal;
use querent_synapse::{callbacks::EventType, config::Config};
use storage::Storage;

#[derive(Clone, Copy, Debug, Default)]
pub struct Trigger {
	pub retry_count: usize,
}

#[derive(Debug)]
struct ControlLoop;

pub struct PipelineSettings {
	pub qflow_id: String,
	pub event_storages: HashMap<EventType, Arc<dyn Storage>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub qflow_config: Config,
}

pub struct SemanticPipeline {
	// Pipeline settings
	//pub setting: PipelineSettings,
	// terimatesignal to kill actors in the pipeline.
	pub terminate_sig: TerimateSignal,
	// Statistics about the event processing system.
	pub statistics: IndexingStatistics,
}

impl SemanticPipeline {
	pub fn new() -> Self {
		Self { terminate_sig: TerimateSignal::default(), statistics: IndexingStatistics::default() }
	}
}

#[async_trait]
impl Actor for SemanticPipeline {
	type ObservableState = IndexingStatistics;

	fn observable_state(&self) -> Self::ObservableState {
		self.statistics.clone()
	}

	fn name(&self) -> String {
		"SemanticKnowledgePipeline".to_string()
	}

	async fn initialize(&mut self, ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		self.handle(Trigger::default(), ctx).await?;
		self.handle(ControlLoop, ctx).await?;
		Ok(())
	}

	async fn finalize(
		&mut self,
		_exit_status: &ActorExitStatus,
		_ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		//self.run_pipeline_observations(ctx);
		Ok(())
	}
}

#[async_trait]
impl Handler<ControlLoop> for SemanticPipeline {
	type Reply = ();
	async fn handle(
		&mut self,
		_control_loop: ControlLoop,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		//self.run_pipeline_observations(ctx);
		//self.perform_health_check(ctx).await?;
		//ctx.schedule_self_msg(HEALTH_CHECK_INTERVAL, control_loop)
		//    .await;
		Ok(())
	}
}

#[async_trait]
impl Handler<Trigger> for SemanticPipeline {
	type Reply = ();
	async fn handle(
		&mut self,
		_trigger: Trigger,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		// TODO trigger the pipeline
		Ok(())
	}
}
