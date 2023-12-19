use actors::{Actor, ActorContext, ActorExitStatus, Handler, MessageBus, QueueCapacity};
use async_trait::async_trait;
use common::RuntimeType;
use std::sync::Arc;
use tokio::runtime::Handle;

use crate::{storage::StorageMapper, EventLock, EventStreamerCounters, EventsBatch, NewEventLock};

pub struct EventStreamer {
	qflow_id: String,
	storage_mapper_messagebus: MessageBus<StorageMapper>,
	timestamp: u64,
	counters: Arc<EventStreamerCounters>,
	publish_event_lock: EventLock,
}

impl EventStreamer {
	pub fn new(
		qflow_id: String,
		storage_mapper_messagebus: MessageBus<StorageMapper>,
		timestamp: u64,
	) -> Self {
		Self {
			qflow_id,
			storage_mapper_messagebus,
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
		QueueCapacity::Bounded(10)
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
		let events = message.events;
		self.counters.increment_events_received(events.len() as u64);
		self.timestamp = message.timestamp;
		println!("EventStreamer received {} events", events.len());
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
