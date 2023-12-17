use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

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

    pub fn increment_events_received(&self) {
        self.events_received.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_events_processed(&self) {
        self.events_processed.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_batches_received(&self) {
        self.batches_received.fetch_add(1, Ordering::SeqCst);
    }
}
