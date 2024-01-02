use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
	indexer::Indexer, EventStreamer, IndexingStatistics, Qflow, SourceActor, StorageMapper,
};
use actors::{
	Actor, ActorContext, ActorExitStatus, ActorHandle, Handler, Health, MessageBus, QueueCapacity,
	Supervisable, HEARTBEAT,
};
use async_trait::async_trait;
use common::{PubSubBroker, TerimateSignal};
use querent_synapse::{callbacks::EventType, querent::Workflow};
use storage::Storage;
use tokio::{sync::Semaphore, time::Instant};
use tracing::{debug, error, info};

static SPAWN_PIPELINE_SEMAPHORE: Semaphore = Semaphore::const_new(10);
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(600);

pub(crate) fn wait_time(retry_count: usize) -> Duration {
	// Protect against a `retry_count` that will lead to an overflow.
	let max_power = (retry_count as u32).min(31);
	Duration::from_secs(2u64.pow(max_power)).min(MAX_RETRY_DELAY)
}

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
	pub qflow: Workflow,

	pub pub_sub_handler: PubSubBroker,
}

struct PipelineHandlers {
	pub _qflow_message_bus: MessageBus<SourceActor>,
	pub qflow_handler: ActorHandle<SourceActor>,
	pub event_streamer_handler: ActorHandle<EventStreamer>,
	pub indexer_handler: ActorHandle<Indexer>,
	pub storage_mapper_handler: ActorHandle<StorageMapper>,
	pub next_progress_check: Instant,
}

impl PipelineHandlers {
	pub fn check_for_progress(&mut self) -> bool {
		let now = Instant::now();
		let check_for_progress = now > self.next_progress_check;
		if check_for_progress {
			self.next_progress_check = now + *HEARTBEAT;
		}
		check_for_progress
	}
}
pub struct SemanticPipeline {
	// Pipeline settings
	pub settings: PipelineSettings,
	// terimatesignal to kill actors in the pipeline.
	pub terminate_sig: TerimateSignal,
	// Statistics about the event processing system.
	pub statistics: IndexingStatistics,
	/// Handlers for the pipeline actors
	handlers: Option<PipelineHandlers>,
}

impl SemanticPipeline {
	pub fn new(settings: PipelineSettings) -> Self {
		Self {
			settings,
			terminate_sig: TerimateSignal::default(),
			statistics: IndexingStatistics::default(),
			handlers: None,
		}
	}

	fn actor_handlers(&self) -> Vec<&dyn Supervisable> {
		if let Some(handles) = &self.handlers {
			let supervisables: Vec<&dyn Supervisable> = vec![
				&handles.qflow_handler,
				&handles.event_streamer_handler,
				&handles.indexer_handler,
				&handles.storage_mapper_handler,
			];
			supervisables
		} else {
			Vec::new()
		}
	}

	fn healthcheck(&self, check_for_progress: bool) -> Health {
		let mut healthy_actors: Vec<&str> = Default::default();
		let mut failure_or_unhealthy_actors: Vec<&str> = Default::default();
		let mut success_actors: Vec<&str> = Default::default();
		for supervisable in self.actor_handlers() {
			match supervisable.check_health(check_for_progress) {
				Health::Healthy => {
					// At least one other actor is running.
					healthy_actors.push(supervisable.name());
				},
				Health::FailureOrUnhealthy => {
					failure_or_unhealthy_actors.push(supervisable.name());
				},
				Health::Success => {
					success_actors.push(supervisable.name());
				},
			}
		}

		if !failure_or_unhealthy_actors.is_empty() {
			error!(
				qflow_id=?self.settings.qflow_id,
				healthy_actors=?healthy_actors,
				failed_or_unhealthy_actors=?failure_or_unhealthy_actors,
				success_actors=?success_actors,
				"Indexing pipeline failure."
			);
			return Health::FailureOrUnhealthy;
		}
		if healthy_actors.is_empty() {
			// All the actors finished successfully.
			info!(
				qflow_id=?self.settings.qflow_id,
				"Semantic pipeline success."
			);
			return Health::Success;
		}
		// No error at this point and there are still some actors running.
		debug!(
			qflow_id=?self.settings.qflow_id,
			healthy_actors=?healthy_actors,
			failed_or_unhealthy_actors=?failure_or_unhealthy_actors,
			success_actors=?success_actors,
			"Semantic pipeline running."
		);
		Health::Healthy
	}

	fn run_pipeline_observations(&mut self, ctx: &ActorContext<Self>) {
		let Some(handles) = &self.handlers else {
			return;
		};
		handles.qflow_handler.refresh_observe();
		handles.event_streamer_handler.refresh_observe();
		handles.indexer_handler.refresh_observe();
		handles.storage_mapper_handler.refresh_observe();
		self.statistics = self.statistics.clone().add_counters(
			&handles.qflow_handler.last_observation(),
			&handles.event_streamer_handler.last_observation(),
			&handles.indexer_handler.last_observation(),
			&handles.storage_mapper_handler.last_observation(),
		);
		ctx.observe(self);
	}
	async fn start_qflow(&mut self, ctx: &ActorContext<Self>) -> anyhow::Result<()> {
		let _spawn_pipeline_permit = ctx
			.protect_future(SPAWN_PIPELINE_SEMAPHORE.acquire())
			.await
			.expect("The semaphore should not be closed.");
		let qflow_id = self.settings.qflow_id.clone();
		self.terminate_sig = ctx.terminate_sig().child();
		info!(
			qflow_id=?qflow_id,
			"spawning semantic pipeline",
		);
		let (source_message_bus, source_inbox) = ctx
			.spawn_ctx()
			.create_messagebus::<SourceActor>("SourceActor", QueueCapacity::Unbounded);
		let current_timestamp = chrono::Utc::now().timestamp_millis() as u64;
		// Storage mapper actor
		let storage_mapper = StorageMapper::new(
			qflow_id.clone(),
			current_timestamp,
			self.settings.event_storages.clone(),
		);
		let (storage_mapper_mailbox, storage_mapper_inbox) = ctx
			.spawn_actor()
			.set_terminate_sig(self.terminate_sig.clone())
			.spawn(storage_mapper);

		// Indexer actor
		let indexer =
			Indexer::new(qflow_id.clone(), current_timestamp, self.settings.index_storages.clone());
		let (indexer_messagebus, indexer_inbox) =
			ctx.spawn_actor().set_terminate_sig(self.terminate_sig.clone()).spawn(indexer);

		// Event streamer actor
		let event_streamer = EventStreamer::new(
			qflow_id.clone(),
			storage_mapper_mailbox,
			indexer_messagebus,
			current_timestamp,
		);
		let (event_streamer_messagebus, event_streamer_inbox) = ctx
			.spawn_actor()
			.set_terminate_sig(self.terminate_sig.clone())
			.spawn(event_streamer);

		// Qflow actor
		let qflow_source = Qflow::new(qflow_id.clone(), self.settings.qflow.clone());
		let qflow_source_actor =
			SourceActor { source: Box::new(qflow_source), event_streamer_messagebus };
		let (qflow_message_bus, qflow_inbox) = ctx
			.spawn_actor()
			.set_messagebuses(source_message_bus, source_inbox)
			.set_terminate_sig(self.terminate_sig.clone())
			.spawn(qflow_source_actor);
		self.handlers = Some(PipelineHandlers {
			_qflow_message_bus: qflow_message_bus,
			qflow_handler: qflow_inbox,
			event_streamer_handler: event_streamer_inbox,
			indexer_handler: indexer_inbox,
			storage_mapper_handler: storage_mapper_inbox,
			next_progress_check: Instant::now() + *HEARTBEAT,
		});
		Ok(())
	}

	async fn terminate(&mut self) {
		self.terminate_sig.kill();
		if let Some(handles) = self.handlers.take() {
			tokio::join!(
				handles.qflow_handler.kill(),
				handles.event_streamer_handler.kill(),
				handles.indexer_handler.kill(),
				handles.storage_mapper_handler.kill(),
			);
		}
	}

	async fn run_health_check(&mut self, ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		let Some(handles) = self.handlers.as_mut() else {
			return Ok(());
		};

		let check_for_progress = handles.check_for_progress();
		let health = self.healthcheck(check_for_progress);
		match health {
			Health::Healthy => {},
			Health::FailureOrUnhealthy => {
				self.terminate().await;
				let first_retry_delay = wait_time(0);
				ctx.schedule_self_msg(first_retry_delay, Trigger { retry_count: 0 }).await;
			},
			Health::Success => {
				return Err(ActorExitStatus::Success);
			},
		}
		Ok(())
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
		ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		self.run_pipeline_observations(ctx);
		Ok(())
	}
}

#[async_trait]
impl Handler<ControlLoop> for SemanticPipeline {
	type Reply = ();
	async fn handle(
		&mut self,
		control_loop: ControlLoop,
		ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.run_pipeline_observations(ctx);
		self.run_health_check(ctx).await?;
		ctx.schedule_self_msg(HEALTH_CHECK_INTERVAL, control_loop).await;
		Ok(())
	}
}

#[async_trait]
impl Handler<Trigger> for SemanticPipeline {
	type Reply = ();
	async fn handle(
		&mut self,
		trigger: Trigger,
		ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		if self.handlers.is_some() {
			return Ok(());
		}
		if let Err(spawn_error) = self.start_qflow(ctx).await {
			let retry_delay = wait_time(trigger.retry_count + 1);
			error!(error = ?spawn_error, retry_count = trigger.retry_count, retry_delay = ?retry_delay, "error while spawning indexing pipeline, retrying after some time");
			ctx.schedule_self_msg(retry_delay, Trigger { retry_count: trigger.retry_count + 1 })
				.await;
		}
		Ok(())
	}
}
