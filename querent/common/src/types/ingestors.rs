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

use std::sync::atomic::AtomicU64;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct IngestorCounters {
	pub total_docs: AtomicU64,
	pub total_megabytes: AtomicU64,
	pub total_ingested_tokens: AtomicU64,
	current_memory_usage: AtomicU64,
}

impl IngestorCounters {
	pub fn new() -> Self {
		Self {
			total_docs: AtomicU64::new(0),
			total_megabytes: AtomicU64::new(0),
			total_ingested_tokens: AtomicU64::new(0),
			current_memory_usage: AtomicU64::new(0),
		}
	}

	pub fn set_current_memory_usage(&self, count: u64) {
		self.current_memory_usage.store(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn get_current_memory_usage(&self) -> u64 {
		self.current_memory_usage.load(std::sync::atomic::Ordering::SeqCst)
	}

	pub fn increment_total_docs(&self, count: u64) {
		self.total_docs.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn increment_total_megabytes(&self, count: u64) {
		self.total_megabytes.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn increment_total_ingested_tokens(&self, count: u64) {
		self.total_ingested_tokens.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}
}
