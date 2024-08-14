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
