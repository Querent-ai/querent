use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	sync::atomic::{AtomicU64, Ordering},
};

use crate::CollectedBytes;

#[derive(Debug, Default, Clone)]
pub struct EventsBatch {
	pub qflow_id: String,
	pub events: HashMap<EventType, Vec<EventState>>,
	pub timestamp: u64,
}

impl EventsBatch {
	pub fn new(
		qflow_id: String,
		events: HashMap<EventType, Vec<EventState>>,
		timestamp: u64,
	) -> Self {
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

	pub fn events(&self) -> HashMap<EventType, Vec<EventState>> {
		self.events.clone()
	}
}

#[derive(Debug, Serialize, Default, Deserialize)]
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

	pub fn increment_events_processed(&self, count: u64) {
		self.events_processed.fetch_add(count, Ordering::SeqCst);
	}

	pub fn increment_batches_received(&self) {
		self.batches_received.fetch_add(1, Ordering::SeqCst);
	}
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CollectionCounter {
	pub total_docs: AtomicU64,
	pub ext_counter_map: HashMap<String, u64>,
}

impl CollectionCounter {
	pub fn new() -> Self {
		Self { total_docs: AtomicU64::new(0), ext_counter_map: HashMap::new() }
	}

	pub fn increment_total_docs(&self) {
		self.total_docs.fetch_add(1, Ordering::SeqCst);
	}

	pub fn increment_ext_counter(&mut self, ext: &String) {
		let counter = self.ext_counter_map.entry(ext.clone()).or_insert(0);
		*counter += 1;
	}
}

#[derive(Debug, Default, Clone)]
pub struct CollectionBatch {
	pub file: String,
	pub ext: String,
	pub bytes: Vec<CollectedBytes>,
}

impl CollectionBatch {
	pub fn new(file: &String, ext: &String, bytes: Vec<CollectedBytes>) -> Self {
		Self { file: file.clone(), bytes, ext: ext.clone() }
	}

	pub fn file(&self) -> String {
		self.file.clone()
	}

	pub fn events(&self) -> &Vec<CollectedBytes> {
		&self.bytes
	}

	pub fn ext(&self) -> String {
		self.ext.clone()
	}
}

// Define an enumeration for different event types
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum EventType {
	Graph,
	Vector,
	Success,
	Failure,
}
// Define a structure to represent the state of an event
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EventState {
	pub event_type: EventType,
	pub timestamp: f64,
	pub payload: String,
	pub file: String,
	pub doc_source: String,
	pub image_id: Option<String>,
}
