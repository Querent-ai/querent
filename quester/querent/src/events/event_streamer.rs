use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::RuntimeType;
use serde::Serialize;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::runtime::Handle;

use crate::{EventLock, EventsBatch};

#[derive(Debug, Serialize)]
pub struct EventStreamerCounters {
    pub events_received: AtomicU64,
    pub events_processed: AtomicU64,
    pub batches_received: AtomicU64,
}

impl EventStreamerCounters {
    pub fn new() -> Self {
        Self {
            events_received: AtomicU64::new(0),
            events_processed: AtomicU64::new(0),
            batches_received: AtomicU64::new(0),
        }
    }

    pub fn increment_events_received(&self, count: u64) {
        self.events_received.fetch_add(count, Ordering::SeqCst);
    }

    pub fn increment_events_processed(&self) {
        self.events_processed.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_batches_received(&self) {
        self.batches_received.fetch_add(1, Ordering::SeqCst);
    }
}

pub struct EventStreamer {
    //event_mapper: Arc<dyn EventMapper>,
    timestamp: u64,
    counters: Arc<EventStreamerCounters>,
    publish_lock: EventLock,
}

impl EventStreamer {
    pub fn new() -> Self {
        Self {
            //event_mapper,
            timestamp: 0,
            counters: Arc::new(EventStreamerCounters::new()),
            publish_lock: EventLock::default(),
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

    pub fn get_publish_lock(&self) -> EventLock {
        self.publish_lock.clone()
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
        _ctx: &ActorContext<Self>,
    ) -> anyhow::Result<()> {
        match exit_status {
            ActorExitStatus::DownstreamClosed
            | ActorExitStatus::Killed
            | ActorExitStatus::Failure(_)
            | ActorExitStatus::Panicked => return Ok(()),
            ActorExitStatus::Quit | ActorExitStatus::Success => {
                println!("EventStreamer exiting with success");
                //let _ = ctx.send_exit_with_success(&self.indexer_messagebus).await;
            }
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
        //self.publish_lock.publish(events_processed).await?;
        println!("EventStreamer received {} events", events.len());
        ctx.record_progress();
        Ok(())
    }
}
