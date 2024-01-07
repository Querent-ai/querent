use querent_synapse::callbacks::EventType;
use serde::Serialize;
use std::{collections::HashMap, sync::atomic::AtomicU64};

#[derive(Debug, Serialize)]
pub struct StorageMapperCounters {
	pub total: AtomicU64,
	pub event_count_map: HashMap<EventType, AtomicU64>,
	pub event_to_storage_map: HashMap<EventType, AtomicU64>,
}

impl StorageMapperCounters {
	pub fn new() -> Self {
		let mut current_event_hashmap = HashMap::new();
		current_event_hashmap.insert(EventType::Graph, AtomicU64::new(0));
		current_event_hashmap.insert(EventType::Vector, AtomicU64::new(0));
		Self {
			total: AtomicU64::new(0),
			event_count_map: current_event_hashmap,
			event_to_storage_map: HashMap::new(),
		}
	}

	pub fn increment_total(&self, count: u64) {
		self.total.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn increment_event_count(&self, event_type: EventType, count: u64) {
		let counter = self.event_count_map.get(&event_type);
		if let Some(counter) = counter {
			counter.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
		}
	}

	pub fn increment_event_to_storage(&self, event_type: EventType, count: u64) {
		let counter = self.event_to_storage_map.get(&event_type);
		if let Some(counter) = counter {
			counter.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
		}
	}
}
