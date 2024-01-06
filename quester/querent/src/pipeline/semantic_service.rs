use crate::{
	IndexingStatistics, MessageStateBatches, PipelineSettings, SemanticPipeline, ShutdownPipe,
};
use actors::{
	Actor, ActorContext, ActorExitStatus, ActorHandle, ActorState, Handler, Healthz, MessageBus,
	Observation, HEARTBEAT,
};
use async_trait::async_trait;
use cluster::Cluster;
use common::PubSubBroker;
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
};
use tracing::{error, info};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticServiceCounters {
	pub num_running_pipelines: usize,
	pub num_successful_pipelines: usize,
	pub num_failed_pipelines: usize,
}

struct PipelineHandle {
	mailbox: MessageBus<SemanticPipeline>,
	handle: ActorHandle<SemanticPipeline>,
	pipeline_id: String,
}

pub struct SemanticService {
	node_id: String,
	cluster: Cluster,
	semantic_pipelines: HashMap<String, PipelineHandle>,
	pubsub_broker: PubSubBroker,
	counters: SemanticServiceCounters,
}

impl Debug for SemanticService {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SemanticService")
			.field("node_id", &self.node_id)
			.field("cluster_id", &self.cluster.cluster_id())
			.finish()
	}
}

impl SemanticService {
	pub fn new(node_id: String, cluster: Cluster, pubsub_broker: PubSubBroker) -> Self {
		Self {
			node_id,
			cluster,
			semantic_pipelines: HashMap::new(),
			pubsub_broker,
			counters: SemanticServiceCounters::default(),
		}
	}
	async fn self_supervise(&mut self) -> Result<(), ActorExitStatus> {
		self.semantic_pipelines.retain(|qflow_id, pipeline_handle| {
			match pipeline_handle.handle.state() {
				ActorState::Idle | ActorState::Paused | ActorState::Processing => true,
				ActorState::Success => {
					info!(
						qflow_id=%qflow_id,
						"Indexing pipeline exited successfully."
					);
					self.counters.num_successful_pipelines += 1;
					self.counters.num_running_pipelines -= 1;
					false
				},
				ActorState::Failure => {
					// This should never happen: Indexing Pipelines are not supposed to fail,
					// and are themselves in charge of supervising the pipeline actors.
					error!(
						qflow_id=%qflow_id,
						"Indexing pipeline exited with failure. This should never happen."
					);
					self.counters.num_failed_pipelines += 1;
					self.counters.num_running_pipelines -= 1;
					false
				},
			}
		});
		let _pipeline_metrics: HashMap<&String, IndexingStatistics> = self
			.semantic_pipelines
			.values()
			.filter_map(|pipeline_handle| {
				let indexing_statistics = pipeline_handle.handle.last_observation();
				Some((&pipeline_handle.pipeline_id, indexing_statistics))
			})
			.collect();
		Ok(())
	}

	async fn observe_pipeline(
		&mut self,
		pipeline_id: String,
	) -> Result<Observation<IndexingStatistics>, anyhow::Error> {
		let pipeline_handle = &self
			.semantic_pipelines
			.get(&pipeline_id)
			.ok_or(anyhow::anyhow!("Semantic pipeline `{}` not found.", pipeline_id))?
			.handle;
		let observation = pipeline_handle.observe().await;
		Ok(observation)
	}

	async fn spawn_pipeline(
		&mut self,
		ctx: &ActorContext<Self>,
		settings: PipelineSettings,
		pipeline_id: String,
	) -> Result<String, anyhow::Error> {
		self.spawn_pipeline_inner(ctx, pipeline_id.clone(), settings).await?;
		Ok(pipeline_id)
	}

	async fn spawn_pipeline_inner(
		&mut self,
		ctx: &ActorContext<Self>,
		pipeline_id: String,
		settings: PipelineSettings,
	) -> Result<(), anyhow::Error> {
		if self.semantic_pipelines.contains_key(&pipeline_id) {
			return Err(anyhow::anyhow!("Semantic pipeline `{}` already exists.", pipeline_id));
		}
		let semantic_pipe = SemanticPipeline::new(settings, self.pubsub_broker.clone());
		let (pipeline_mailbox, pipeline_handle) = ctx.spawn_actor().spawn(semantic_pipe);
		let pipeline_handle = PipelineHandle {
			mailbox: pipeline_mailbox,
			handle: pipeline_handle,
			pipeline_id: pipeline_id.clone(),
		};
		self.semantic_pipelines.insert(pipeline_id, pipeline_handle);
		self.counters.num_running_pipelines += 1;
		Ok(())
	}
}

#[derive(Debug)]
struct SuperviseLoop;

#[async_trait]
impl Handler<SuperviseLoop> for SemanticService {
	type Reply = ();

	async fn handle(
		&mut self,
		_message: SuperviseLoop,
		ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.self_supervise().await?;
		ctx.schedule_self_msg(*HEARTBEAT, SuperviseLoop).await;
		Ok(())
	}
}

#[async_trait]
impl Actor for SemanticService {
	type ObservableState = SemanticServiceCounters;

	fn observable_state(&self) -> Self::ObservableState {
		self.counters.clone()
	}

	async fn initialize(&mut self, ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		self.handle(SuperviseLoop, ctx).await
	}
}

#[derive(Debug)]
pub struct ObservePipeline {
	pub pipeline_id: String,
}

#[derive(Clone, Debug)]
pub struct ShutdownPipeline {
	pub pipeline_id: String,
}

pub struct SpawnPipeline {
	pub settings: PipelineSettings,
	pub pipeline_id: String,
}

#[async_trait]
impl Handler<ObservePipeline> for SemanticService {
	type Reply = Result<Observation<IndexingStatistics>, anyhow::Error>;

	async fn handle(
		&mut self,
		msg: ObservePipeline,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let observation = self.observe_pipeline(msg.pipeline_id).await;
		Ok(observation)
	}
}

#[async_trait]
impl Handler<SpawnPipeline> for SemanticService {
	type Reply = Result<String, anyhow::Error>;
	async fn handle(
		&mut self,
		message: SpawnPipeline,
		ctx: &ActorContext<Self>,
	) -> Result<Result<String, anyhow::Error>, ActorExitStatus> {
		Ok(self.spawn_pipeline(ctx, message.settings, message.pipeline_id).await)
	}
}

#[async_trait]
impl Handler<Healthz> for SemanticService {
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
impl Handler<ShutdownPipeline> for SemanticService {
	type Reply = ();

	async fn handle(
		&mut self,
		message: ShutdownPipeline,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let pipeline_handle = &self
			.semantic_pipelines
			.get(&message.pipeline_id)
			.ok_or(anyhow::anyhow!("Semantic pipeline `{}` not found.", message.pipeline_id))?;
		let shutdown_message = ShutdownPipe { pipeline_id: message.pipeline_id.clone() };
		pipeline_handle.mailbox.send_message(shutdown_message).await?;
		Ok(())
	}
}

#[async_trait]
impl Handler<MessageStateBatches> for SemanticService {
	type Reply = ();

	async fn handle(
		&mut self,
		_message: MessageStateBatches,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		Ok(())
	}
}
