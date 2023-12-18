use querent_synapse::callbacks::{EventState, EventType};
use serde::Serialize;
use std::{
	collections::HashMap,
	sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Default)]
pub struct EventsBatch {
	pub qflow_id: String,
	pub events: HashMap<EventType, EventState>,
	pub timestamp: u64,
}

impl EventsBatch {
	pub fn new(qflow_id: String, events: HashMap<EventType, EventState>, timestamp: u64) -> Self {
		Self { qflow_id, events, timestamp }
	}

	pub fn is_empty(&self) -> bool {
		self.events.is_empty()
	}

	pub fn len(&self) -> usize {
		self.events.len()
	}

	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn qflow_id(&self) -> String {
		self.qflow_id.clone()
	}

	pub fn events(&self) -> HashMap<EventType, EventState> {
		self.events.clone()
	}
}

#[derive(Debug, Serialize)]
pub struct EventsCounter {
	pub qflow_id: String,
	pub total: AtomicU64,
	pub processed: AtomicU64,
}

impl EventsCounter {
	pub fn new(qflow_id: String) -> Self {
		Self { qflow_id, total: AtomicU64::new(0), processed: AtomicU64::new(0) }
	}

	pub fn increment_total(&self) {
		self.total.fetch_add(1, Ordering::SeqCst);
	}

	pub fn increment_processed(&self, count: u64) {
		self.processed.fetch_add(count, Ordering::SeqCst);
	}
}

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
