use actors::{Actor, ActorContext, ActorExitStatus, Handler, MessageBus, QueueCapacity};
use async_trait::async_trait;
use common::{EventStreamerCounters, EventsBatch, RuntimeType};
use querent_synapse::callbacks::EventType;
use std::sync::Arc;
use tokio::runtime::Handle;
use tracing::error;

use crate::{
	indexer::Indexer,
	storage::{ContextualEmbeddings, ContextualTriples, StorageMapper},
	EventLock, IndexerKnowledge, NewEventLock,
};

pub struct EventStreamer {
	qflow_id: String,
	storage_mapper_messagebus: MessageBus<StorageMapper>,
	indexer_messagebus: MessageBus<Indexer>,
	timestamp: u64,
	counters: Arc<EventStreamerCounters>,
	publish_event_lock: EventLock,
}

impl EventStreamer {
	pub fn new(
		qflow_id: String,
		storage_mapper_messagebus: MessageBus<StorageMapper>,
		indexer_messagebus: MessageBus<Indexer>,
		timestamp: u64,
	) -> Self {
		Self {
			qflow_id,
			storage_mapper_messagebus,
			indexer_messagebus,
			timestamp,
			counters: Arc::new(EventStreamerCounters::new()),
			publish_event_lock: EventLock::default(),
		}
	}

	pub fn get_counters(&self) -> Arc<EventStreamerCounters> {
		self.counters.clone()
	}

	pub fn get_timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn set_timestamp(&mut self, timestamp: u64) {
		self.timestamp = timestamp;
	}

	pub fn get_publish_event_lock(&self) -> EventLock {
		self.publish_event_lock.clone()
	}

	pub fn get_qflow_id(&self) -> String {
		self.qflow_id.clone()
	}
}

#[async_trait]
impl Actor for EventStreamer {
	type ObservableState = Arc<EventStreamerCounters>;

	fn observable_state(&self) -> Self::ObservableState {
		self.counters.clone()
	}

	fn queue_capacity(&self) -> QueueCapacity {
		QueueCapacity::Unbounded
	}

	fn runtime_handle(&self) -> Handle {
		RuntimeType::Blocking.get_runtime_handle()
	}

	#[inline]
	fn yield_after_each_message(&self) -> bool {
		false
	}

	async fn finalize(
		&mut self,
		exit_status: &ActorExitStatus,
		ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		match exit_status {
			ActorExitStatus::DownstreamClosed |
			ActorExitStatus::Killed |
			ActorExitStatus::Failure(_) |
			ActorExitStatus::Panicked => return Ok(()),
			ActorExitStatus::Quit | ActorExitStatus::Success => {
				log::info!("EventStreamer exiting with success");
				let _ = ctx.send_exit_with_success(&self.storage_mapper_messagebus).await;
				let _ = ctx.send_exit_with_success(&self.indexer_messagebus).await;
			},
		}
		Ok(())
	}
}

#[async_trait]
impl Handler<EventsBatch> for EventStreamer {
	type Reply = ();

	async fn handle(
		&mut self,
		message: EventsBatch,
		ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.counters.increment_batches_received();
		let grouped_events = message.events.clone();
		self.counters.increment_events_received(message.events.len() as u64);
		self.timestamp = message.timestamp;

		// Send grouped events to StorageMapper
		for (event_type, event_states) in grouped_events {
			match event_type {
				EventType::Graph => {
					let contextual_triples: ContextualTriples =
						ContextualTriples::new(self.qflow_id.clone(), event_states, self.timestamp);
					let mapper_res = ctx
						.send_message(&self.storage_mapper_messagebus, contextual_triples.clone())
						.await;
					match mapper_res {
						Ok(_) => {},
						Err(e) => {
							error!("Error sending message to StorageMapper: {:?}", e);
						},
					}
					let indexer_knowledge = IndexerKnowledge::new(
						self.qflow_id.clone(),
						self.timestamp,
						contextual_triples.event_payload(),
					);
					let indexer_res =
						ctx.send_message(&self.indexer_messagebus, indexer_knowledge).await;
					match indexer_res {
						Ok(_) => {},
						Err(e) => {
							error!("Error sending message to Indexer: {:?}", e);
						},
					}
				},
				EventType::Vector => {
					let contextual_embeddings = ContextualEmbeddings::new(
						self.qflow_id.clone(),
						event_states,
						self.timestamp,
					);
					let vec_res = ctx
						.send_message(&self.storage_mapper_messagebus, contextual_embeddings)
						.await;
					match vec_res {
						Ok(_) => {},
						Err(e) => {
							error!("Error sending message to StorageMapper: {:?}", e);
						},
					}
				},
				_ => {},
			}
		}
		self.counters.increment_events_processed(message.events.len() as u64);
		ctx.record_progress();
		Ok(())
	}
}

#[async_trait]
impl Handler<NewEventLock> for EventStreamer {
	type Reply = ();

	async fn handle(
		&mut self,
		message: NewEventLock,
		ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		let NewEventLock(publish_event_lock) = &message;
		self.publish_event_lock = publish_event_lock.clone();
		ctx.send_message(&self.storage_mapper_messagebus, message).await?;
		Ok(())
	}
}
