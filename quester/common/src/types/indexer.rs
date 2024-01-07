use std::sync::atomic::AtomicU64;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct IndexerCounters {
	pub total: AtomicU64,
	pub total_documents_indexed: AtomicU64,
	pub total_sentences_indexed: AtomicU64,
	pub total_subjects_indexed: AtomicU64,
	pub total_predicates_indexed: AtomicU64,
	pub total_objects_indexed: AtomicU64,
}

impl IndexerCounters {
	pub fn new() -> Self {
		Self {
			total: AtomicU64::new(0),
			total_documents_indexed: AtomicU64::new(0),
			total_sentences_indexed: AtomicU64::new(0),
			total_subjects_indexed: AtomicU64::new(0),
			total_predicates_indexed: AtomicU64::new(0),
			total_objects_indexed: AtomicU64::new(0),
		}
	}

	pub fn increment_total(&self, count: u64) {
		self.total.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
	}

	pub fn increment_total_documents_indexed(&self, count: u64) {
		self.total_documents_indexed
			.fetch_add(count, std::sync::atomic::Ordering::SeqCst);
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
