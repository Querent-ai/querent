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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use serde::Serialize;
use std::{collections::HashMap, sync::atomic::AtomicU64};

use crate::EventType;

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
