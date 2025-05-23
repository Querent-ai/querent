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

use crate::{
	indexer::Indexer, ingest::ingestor_service::IngestorService, Collector, EngineRunner,
	EventStreamer, SourceActor, StorageMapper,
};
use actors::{
	Actor, ActorContext, ActorExitStatus, ActorHandle, Handler, Health, QueueCapacity,
	Supervisable, HEARTBEAT,
};
use async_trait::async_trait;
use common::{EventType, PubSubBroker, TerimateSignal};
use engines::Engine;
use proto::semantics::{IndexingStatistics, IngestedTokens};
use sources::DataSource;
use std::{collections::HashMap, sync::Arc, time::Duration};
use storage::{SecretStorage, Storage};
use tokio::{
	sync::{mpsc, Semaphore},
	time::Instant,
};
use tracing::{debug, error, info};

static SPAWN_PIPELINE_SEMAPHORE: Semaphore = Semaphore::const_new(10);
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(5);
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

#[derive(Clone, Debug)]
pub struct PipelineSettings {
	pub data_sources: Vec<Arc<dyn sources::DataSource>>,
	pub event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub secret_store: Arc<dyn SecretStorage>,
	pub engine: Arc<dyn Engine>,
}

struct PipelineHandlers {
	pub engine_handler: ActorHandle<SourceActor>,
	pub event_streamer_handler: ActorHandle<EventStreamer>,
	pub indexer_handler: ActorHandle<Indexer>,
	pub storage_mapper_handler: ActorHandle<StorageMapper>,
	pub collection_handlers: Vec<ActorHandle<SourceActor>>,
	pub ingestor_handler: ActorHandle<IngestorService>,
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
	// id of the pipeline.
	pub id: String,
	// Dynamic enging running the pipeline.
	pub engine: Arc<dyn Engine>,
	// Data sources
	pub data_sources: Vec<Arc<dyn sources::DataSource>>,
	// Token sender
	pub token_sender: Option<mpsc::Sender<IngestedTokens>>,
	// Event storages
	pub event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	// Index storages
	pub index_storages: Vec<Arc<dyn Storage>>,
	// terimatesignal to kill actors in the pipeline.
	pub terminate_sig: TerimateSignal,
	// Statistics about the event processing system.
	pub statistics: IndexingStatistics,
	// Handlers for the pipeline actors
	handlers: Option<PipelineHandlers>,
	// pubsub broker
	pub pubsub_broker: PubSubBroker,
	retry_count: usize,
}

impl SemanticPipeline {
	pub fn new(
		id: String,
		engine: Arc<dyn Engine>,
		data_sources: Vec<Arc<dyn DataSource>>,
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		index_storages: Vec<Arc<dyn Storage>>,
		pubsub_broker: PubSubBroker,
	) -> Self {
		Self {
			id,
			engine,
			data_sources,
			event_storages,
			index_storages,
			terminate_sig: TerimateSignal::default(),
			statistics: IndexingStatistics::default(),
			handlers: None,
			pubsub_broker,
			token_sender: None,
			retry_count: 0,
		}
	}

	fn actor_handlers(&self) -> Vec<&dyn Supervisable> {
		if let Some(handles) = &self.handlers {
			let mut all_handles: Vec<&dyn Supervisable> = handles
				.collection_handlers
				.iter()
				.map(|handler| handler as &dyn Supervisable)
				.collect();
			let supervisables: Vec<&dyn Supervisable> = vec![
				&handles.engine_handler,
				&handles.event_streamer_handler,
				&handles.indexer_handler,
				&handles.storage_mapper_handler,
				&handles.ingestor_handler,
			];
			all_handles.extend(supervisables);
			all_handles
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
				engine_id=?self.id,
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
				engine_id=?self.id,
				"Semantic pipeline success."
			);
			return Health::Success;
		}
		// No error at this point and there are still some actors running.
		debug!(
			engine_id=?self.id,
			healthy_actors=?healthy_actors,
			failed_or_unhealthy_actors=?failure_or_unhealthy_actors,
			success_actors=?success_actors,
			"Semantic pipeline running."
		);
		Health::Healthy
	}

	async fn run_pipeline_observations(&mut self, ctx: &ActorContext<Self>) {
		let Some(handles) = &self.handlers else {
			return;
		};
		handles.engine_handler.refresh_observe();
		handles.event_streamer_handler.refresh_observe();
		handles.indexer_handler.refresh_observe();
		handles.storage_mapper_handler.refresh_observe();
		handles.collection_handlers.iter().for_each(|handler| handler.refresh_observe());
		handles.ingestor_handler.refresh_observe();
		let mut collection_counters = vec![];
		for handler in &handles.collection_handlers {
			collection_counters.push(handler.last_observation());
		}
		self.statistics = self.statistics.clone().add_counters(
			&handles.engine_handler.last_observation(),
			&handles.event_streamer_handler.last_observation(),
			&handles.indexer_handler.last_observation(),
			&handles.storage_mapper_handler.last_observation(),
			&handles.ingestor_handler.last_observation(),
			collection_counters,
		);
		ctx.observe(self);
	}

	async fn start_engine(&mut self, ctx: &ActorContext<Self>) -> anyhow::Result<()> {
		let (token_sender, token_receiver) = mpsc::channel(10);
		self.token_sender = Some(token_sender.clone());
		let _spawn_pipeline_permit = ctx
			.protect_future(SPAWN_PIPELINE_SEMAPHORE.acquire())
			.await
			.expect("The semaphore should not be closed.");
		let engine_id = self.id.clone();

		self.terminate_sig = ctx.terminate_sig().child();
		info!(
			engine_id=?engine_id,
			"spawning semantic pipeline",
		);

		let current_timestamp = chrono::Utc::now().timestamp_millis() as u64;

		// Storage mapper actor
		let storage_mapper =
			StorageMapper::new(engine_id.clone(), current_timestamp, self.event_storages.clone());
		let (storage_mapper_mailbox, storage_mapper_inbox) = ctx
			.spawn_actor()
			.set_terminate_sig(self.terminate_sig.clone())
			.spawn(storage_mapper);

		// Indexer actor
		let indexer =
			Indexer::new(engine_id.clone(), current_timestamp, self.index_storages.clone());
		let (indexer_messagebus, indexer_inbox) =
			ctx.spawn_actor().set_terminate_sig(self.terminate_sig.clone()).spawn(indexer);

		// Ingestor actor
		let ingestor_service = IngestorService::new(
			engine_id.clone(),
			token_sender.clone(),
			current_timestamp,
			self.terminate_sig.clone(),
		);

		let (ingestor_mailbox, ingestor_inbox) = ctx
			.spawn_actor()
			.set_terminate_sig(self.terminate_sig.clone())
			.spawn(ingestor_service);
		// Event streamer actor
		let event_streamer = EventStreamer::new(
			engine_id.clone(),
			storage_mapper_mailbox,
			indexer_messagebus,
			ingestor_mailbox.clone(),
			current_timestamp,
		);
		let (event_streamer_messagebus, event_streamer_inbox) = ctx
			.spawn_actor()
			.set_terminate_sig(self.terminate_sig.clone())
			.spawn(event_streamer);

		// Start various source actors
		let mut collection_handlers = Vec::new();
		let collector_source = Collector::new(
			engine_id.clone(),
			self.data_sources.clone(),
			self.terminate_sig.clone(),
		);

		let (collector_message_bus, collector_inbox) = ctx
			.spawn_ctx()
			.create_messagebus::<SourceActor>("DataSourceActor", QueueCapacity::Unbounded);
		let collector_actor = SourceActor {
			source: Box::new(collector_source),
			event_streamer_messagebus: event_streamer_messagebus.clone(),
			ingestor_messagebus: ingestor_mailbox.clone(),
		};
		let (_, collector_handle) = ctx
			.spawn_actor()
			.set_messagebuses(collector_message_bus, collector_inbox)
			.set_terminate_sig(self.terminate_sig.clone())
			.spawn(collector_actor);
		collection_handlers.push(collector_handle);

		info!("Starting the collector actor 📚");
		info!("Starting the engine actor 🧠");
		info!("Starting the event streamer actor ⇵");
		info!("Starting the indexer actor 📦");
		info!("Starting the storage mapper actor 📦");
		info!("Starting the source actor 🔗");
		info!("Starting the Ingestor actor 📦");

		// EngineRunner actor
		let engine_source = EngineRunner::new(
			self.id.clone(),
			self.engine.clone(),
			token_receiver,
			self.terminate_sig.clone(),
		);
		let (engine_message_bus, engine_inbox) = ctx
			.spawn_ctx()
			.create_messagebus::<SourceActor>("EngineSourceActor", QueueCapacity::Unbounded);
		let engine_source_actor = SourceActor {
			source: Box::new(engine_source),
			event_streamer_messagebus,
			ingestor_messagebus: ingestor_mailbox,
		};
		let (_, engine_handle) = ctx
			.spawn_actor()
			.set_messagebuses(engine_message_bus, engine_inbox)
			.set_terminate_sig(self.terminate_sig.clone())
			.spawn(engine_source_actor);
		self.handlers = Some(PipelineHandlers {
			engine_handler: engine_handle,
			event_streamer_handler: event_streamer_inbox,
			indexer_handler: indexer_inbox,
			storage_mapper_handler: storage_mapper_inbox,
			next_progress_check: Instant::now() + *HEARTBEAT,
			ingestor_handler: ingestor_inbox,
			collection_handlers,
		});
		Ok(())
	}

	async fn terminate(&mut self) {
		self.terminate_sig.kill();
		if let Some(handles) = self.handlers.take() {
			tokio::join!(
				handles.engine_handler.kill(),
				handles.event_streamer_handler.kill(),
				handles.indexer_handler.kill(),
				handles.storage_mapper_handler.kill(),
				async {
					for handler in handles.collection_handlers {
						handler.kill().await;
					}
				},
				handles.ingestor_handler.kill(),
			);
		}
	}

	async fn run_health_check(&mut self, ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		let Some(handles) = self.handlers.as_mut() else {
			return Ok(());
		};

		let check_for_progress = handles.check_for_progress();
		if !check_for_progress {
			return Ok(());
		}
		let health = self.healthcheck(check_for_progress);
		match health {
			Health::Healthy => {},
			Health::FailureOrUnhealthy => {
				if self.retry_count > 3 {
					self.terminate().await;
					return Err(ActorExitStatus::Failure(
						anyhow::anyhow!("Semantic pipeline failure.").into(),
					));
				}
				let first_retry_delay = wait_time(0);
				self.retry_count += 1;
				ctx.schedule_self_msg(first_retry_delay, Trigger { retry_count: self.retry_count });
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
		self.run_pipeline_observations(ctx).await;
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
		self.run_pipeline_observations(ctx).await;
		self.run_health_check(ctx).await?;
		ctx.schedule_self_msg(HEALTH_CHECK_INTERVAL, control_loop);
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
		if let Err(spawn_error) = self.start_engine(ctx).await {
			let retry_delay = wait_time(trigger.retry_count + 1);
			error!(error = ?spawn_error, retry_count = trigger.retry_count, retry_delay = ?retry_delay, "error while spawning indexing pipeline, retrying after some time");
			self.terminate().await;
			return Err(ActorExitStatus::Failure(
				anyhow::anyhow!("error while spawning indexing pipeline, retrying after some time")
					.into(),
			));
		}
		Ok(())
	}
}

#[derive(Clone, Debug)]
pub struct ShutdownPipe {
	pub pipeline_id: String,
}

#[async_trait]
impl Handler<ShutdownPipe> for SemanticPipeline {
	type Reply = ();
	async fn handle(
		&mut self,
		_shutdown_pipe: ShutdownPipe,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		if self.id != _shutdown_pipe.pipeline_id {
			return Ok(());
		}
		self.terminate().await;
		Err(ActorExitStatus::Success)
	}
}

#[async_trait]
impl Handler<IngestedTokens> for SemanticPipeline {
	type Reply = ();
	async fn handle(
		&mut self,
		ingested_tokens: IngestedTokens,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.token_sender
			.as_ref()
			.expect("Token sender should be present.")
			.send(ingested_tokens)
			.await
			.expect("Token sender should not be closed.");

		Ok(())
	}
}
