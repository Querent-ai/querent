use crate::{PipelineErrors, PipelineSettings, SemanticPipeline, ShutdownPipe};
use actors::{
	Actor, ActorContext, ActorExitStatus, ActorHandle, ActorState, Handler, Healthz, MessageBus,
	Observation, HEARTBEAT,
};
use async_trait::async_trait;
use cluster::Cluster;
use common::PubSubBroker;
use proto::semantics::{
	EmptyGetPipelinesMetadata, IndexingStatistics, PipelineMetadata, SendIngestedTokens,
};
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
	sync::Arc,
};
use storage::Storage;
use tracing::{error, info};

#[cfg(feature = "license-check")]
use crate::{get_pipeline_count_by_product, get_total_sources_by_product};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SemanticServiceCounters {
	pub num_running_pipelines: usize,
	pub num_successful_pipelines: usize,
	pub num_failed_pipelines: usize,
}

struct PipelineHandle {
	mailbox: MessageBus<SemanticPipeline>,
	handle: ActorHandle<SemanticPipeline>,
	pipeline_id: String,
	settings: PipelineSettings,
}

pub struct SemanticService {
	node_id: String,
	cluster: Cluster,
	semantic_pipelines: HashMap<String, PipelineHandle>,
	#[allow(unused)]
	secret_store: Arc<dyn Storage>,
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
	pub fn new(
		node_id: String,
		cluster: Cluster,
		pubsub_broker: PubSubBroker,
		secret_store: Arc<dyn Storage>,
	) -> Self {
		Self {
			node_id,
			cluster,
			semantic_pipelines: HashMap::new(),
			pubsub_broker,
			counters: SemanticServiceCounters::default(),
			secret_store,
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
					// This should never happen: Semantic Pipelines are not supposed to fail,
					// and are themselves in charge of supervising the pipeline actors.
					error!(
						qflow_id=%qflow_id,
						"Semantic pipeline exited with failure. This should never happen."
					);
					self.counters.num_failed_pipelines += 1;
					self.counters.num_running_pipelines -= 1;
					false
				},
			}
		});
		let pipeline_metrics: HashMap<&String, IndexingStatistics> = self
			.semantic_pipelines
			.values()
			.filter_map(|pipeline_handle| {
				let indexing_statistics = pipeline_handle.handle.last_observation();
				Some((&pipeline_handle.pipeline_id, indexing_statistics))
			})
			.collect();

		self.cluster.update_semantic_service_metrics(&pipeline_metrics).await;
		Ok(())
	}

	async fn observe_pipeline(
		&mut self,
		pipeline_id: String,
	) -> Result<Observation<IndexingStatistics>, PipelineErrors> {
		let pipeline_handle = &self
			.semantic_pipelines
			.get(&pipeline_id)
			.ok_or(PipelineErrors::PipelineNotFound { pipeline_id })?
			.handle;
		let observation = pipeline_handle.observe().await;
		Ok(observation)
	}

	async fn spawn_pipeline(
		&mut self,
		ctx: &ActorContext<Self>,
		settings: PipelineSettings,
		pipeline_id: String,
	) -> Result<String, PipelineErrors> {
		self.spawn_pipeline_inner(ctx, pipeline_id.clone(), settings).await?;
		Ok(pipeline_id)
	}

	async fn spawn_pipeline_inner(
		&mut self,
		ctx: &ActorContext<Self>,
		pipeline_id: String,
		settings: PipelineSettings,
	) -> Result<(), PipelineErrors> {
		#[cfg(feature = "license-check")]
		{
			let licence_key = self
				.secret_store
				.get_rian_api_key()
				.await
				.map_err(|_e| PipelineErrors::MissingLicenseKey)?;
			if licence_key.is_none() {
				return Err(PipelineErrors::MissingLicenseKey);
			}
			let allowed_pipelines = get_pipeline_count_by_product(licence_key.clone().unwrap())
				.map_err(|e| PipelineErrors::InvalidParams(e.into()))?;
			if self.semantic_pipelines.contains_key(&pipeline_id) {
				return Err(PipelineErrors::PipelineAlreadyExists { pipeline_id });
			}
			if self.semantic_pipelines.len() >= allowed_pipelines {
				return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
					"Maximum number of pipelines allowed for this license key is {}",
					allowed_pipelines
				)));
			}
			let total_sources_allowed_per_pipeline =
				get_total_sources_by_product(licence_key.unwrap())
					.map_err(|e| PipelineErrors::InvalidParams(e.into()))?;
			if settings.data_sources.len() > total_sources_allowed_per_pipeline {
				return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
					"Maximum number of sources allowed per pipeline for this license key is {}",
					total_sources_allowed_per_pipeline
				)));
			}
		}
		if self.semantic_pipelines.contains_key(&pipeline_id) {
			return Err(PipelineErrors::PipelineAlreadyExists { pipeline_id });
		}
		let semantic_pipe = SemanticPipeline::new(
			pipeline_id.clone(),
			settings.engine.clone(),
			settings.data_sources.clone(),
			settings.event_storages.clone(),
			settings.index_storages.clone(),
			self.pubsub_broker.clone(),
		);

		let (pipeline_mailbox, pipeline_handle) = ctx.spawn_actor().spawn(semantic_pipe);
		let pipeline_handle = PipelineHandle {
			mailbox: pipeline_mailbox,
			handle: pipeline_handle,
			settings,
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
		ctx.schedule_self_msg(*HEARTBEAT, SuperviseLoop);
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

	async fn finalize(
		&mut self,
		_exit_status: &ActorExitStatus,
		_ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		// kill all pipelines
		for pipeline_handle in self.semantic_pipelines.values() {
			let shutdown = ShutdownPipe { pipeline_id: pipeline_handle.pipeline_id.clone() };
			let _ = pipeline_handle.mailbox.send_message(shutdown).await;
		}
		Ok(())
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

#[derive(Clone, Debug)]
pub struct SpawnPipeline {
	pub settings: PipelineSettings,
	pub pipeline_id: String,
}

#[async_trait]
impl Handler<ObservePipeline> for SemanticService {
	type Reply = Result<IndexingStatistics, PipelineErrors>;

	async fn handle(
		&mut self,
		msg: ObservePipeline,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let observation = self.observe_pipeline(msg.pipeline_id).await;
		match observation {
			Ok(observation) => Ok(Ok(observation.state)),
			Err(e) => Ok(Err(e)),
		}
	}
}

#[async_trait]
impl Handler<SpawnPipeline> for SemanticService {
	type Reply = Result<String, PipelineErrors>;
	async fn handle(
		&mut self,
		message: SpawnPipeline,
		ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
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
		let pipeline_handle_opt = &self.semantic_pipelines.get(&message.pipeline_id);
		if pipeline_handle_opt.is_none() {
			return Ok(());
		}
		let pipeline_handle_opt = pipeline_handle_opt.unwrap();
		let shutdown_message = ShutdownPipe { pipeline_id: message.pipeline_id.clone() };
		pipeline_handle_opt.mailbox.send_message(shutdown_message).await?;
		Ok(())
	}
}

#[async_trait]
impl Handler<SendIngestedTokens> for SemanticService {
	type Reply = ();

	async fn handle(
		&mut self,
		message: SendIngestedTokens,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let pipeline_handle = &self
			.semantic_pipelines
			.get(&message.pipeline_id)
			.ok_or(anyhow::anyhow!("Semantic pipeline `{}` not found.", message.pipeline_id))?;
		for token in message.tokens {
			pipeline_handle.mailbox.send_message(token).await?;
		}
		Ok(())
	}
}

#[async_trait]
impl Handler<EmptyGetPipelinesMetadata> for SemanticService {
	type Reply = Vec<PipelineMetadata>;

	async fn handle(
		&mut self,
		_message: EmptyGetPipelinesMetadata,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let pipelines_metadata: Vec<PipelineMetadata> = self
			.semantic_pipelines
			.values()
			.map(|pipeline_handle| PipelineMetadata {
				pipeline_id: pipeline_handle.pipeline_id.clone(),
			})
			.collect();
		Ok(pipelines_metadata)
	}
}

#[derive(Clone, Debug)]
pub struct RestartPipeline {
	pub pipeline_id: String,
}

#[async_trait]
impl Handler<RestartPipeline> for SemanticService {
	type Reply = Result<String, PipelineErrors>;

	async fn handle(
		&mut self,
		message: RestartPipeline,
		ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		let pipeline_handle = &self
			.semantic_pipelines
			.get(&message.pipeline_id)
			.ok_or(anyhow::anyhow!("Semantic pipeline `{}` not found.", message.pipeline_id))?;
		let settings = pipeline_handle.settings.clone();
		let pipeline_id = pipeline_handle.pipeline_id.clone();
		let success = ctx
			.send_self_message(ShutdownPipeline { pipeline_id: pipeline_id.clone() })
			.await;
		if success.is_err() {
			return Err(anyhow::anyhow!(
				"Failed to restart pipeline `{}`. Could not shutdown pipeline.",
				message.pipeline_id
			)
			.into());
		}
		Ok(self.spawn_pipeline(ctx, settings, message.pipeline_id).await)
	}
}
