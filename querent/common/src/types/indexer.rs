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
pub struct IndexerCounters {
	pub total: AtomicU64,
	pub total_sentences_indexed: AtomicU64,
	pub total_subjects_indexed: AtomicU64,
	pub total_predicates_indexed: AtomicU64,
	pub total_objects_indexed: AtomicU64,
}

impl IndexerCounters {
	pub fn new() -> Self {
		Self {
			total: AtomicU64::new(0),
			total_sentences_indexed: AtomicU64::new(0),
			total_subjects_indexed: AtomicU64::new(0),
			total_predicates_indexed: AtomicU64::new(0),
			total_objects_indexed: AtomicU64::new(0),
		}
	}

	pub fn increment_total(&self, count: u64) {
		self.total.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn increment_total_sentences_indexed(&self, count: u64) {
		self.total_sentences_indexed
			.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn increment_total_subjects_indexed(&self, count: u64) {
		self.total_subjects_indexed
			.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn increment_total_predicates_indexed(&self, count: u64) {
		self.total_predicates_indexed
			.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn increment_total_objects_indexed(&self, count: u64) {
		self.total_objects_indexed.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}
}
