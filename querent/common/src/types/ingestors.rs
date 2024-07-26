use std::sync::atomic::AtomicU64;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct IngestorCounters {
	pub total_docs: AtomicU64,
	pub total_megabytes: AtomicU64,
	pub total_ingested_tokens: AtomicU64,
	pub current_running_count: AtomicU64,
}

impl IngestorCounters {
	pub fn new() -> Self {
		Self {
			total_docs: AtomicU64::new(0),
			total_megabytes: AtomicU64::new(0),
			total_ingested_tokens: AtomicU64::new(0),
			current_running_count: AtomicU64::new(0),
		}
	}

	pub fn increment_current_running_count(&self, count: u64) {
		self.current_running_count.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn decrement_current_running_count(&self, count: u64) {
		self.current_running_count.fetch_sub(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn get_current_running_count(&self) -> u64 {
		self.current_running_count.load(std::sync::atomic::Ordering::SeqCst)
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
