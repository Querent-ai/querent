use std::sync::{atomic::AtomicU64, Arc};

use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::RuntimeType;
use serde::Serialize;
use storage::Storage;
use tokio::runtime::Handle;

use crate::{ContextualTriples, EventLock, NewEventLock};

#[derive(Debug, Serialize)]
pub struct StorageMapperCounters {
	total: AtomicU64,
	total_documents_indexed: AtomicU64,
	total_sentences_indexed: AtomicU64,
	total_subjects_indexed: AtomicU64,
	total_predicates_indexed: AtomicU64,
	total_objects_indexed: AtomicU64,
}

impl StorageMapperCounters {
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

pub struct Indexer {
	pub qflow_id: String,
	pub timestamp: u64,
	pub counters: Arc<StorageMapperCounters>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub event_lock: EventLock,
}

impl Indexer {
	pub fn new(qflow_id: String, timestamp: u64, index_storages: Vec<Arc<dyn Storage>>) -> Self {
		Self {
			qflow_id,
			timestamp,
			counters: Arc::new(StorageMapperCounters::new()),
			event_lock: EventLock::default(),
			index_storages,
		}
	}

	pub fn get_counters(&self) -> Arc<StorageMapperCounters> {
		self.counters.clone()
	}

	pub fn get_timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn set_timestamp(&mut self, timestamp: u64) {
		self.timestamp = timestamp;
	}

	pub fn get_event_lock(&self) -> EventLock {
		self.event_lock.clone()
	}

	pub fn get_qflow_id(&self) -> String {
		self.qflow_id.clone()
	}
}

#[async_trait]
impl Actor for Indexer {
	type ObservableState = Arc<StorageMapperCounters>;

	fn observable_state(&self) -> Self::ObservableState {
		self.counters.clone()
	}

	fn name(&self) -> String {
		"Indexer".to_string()
	}

	fn queue_capacity(&self) -> QueueCapacity {
		QueueCapacity::Bounded(10)
	}

	fn runtime_handle(&self) -> Handle {
		RuntimeType::Blocking.get_runtime_handle()
	}

	#[inline]
	fn yield_after_each_message(&self) -> bool {
		false
	}

	async fn finalize(
		&mut self,
		exit_status: &ActorExitStatus,
		_ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		match exit_status {
			ActorExitStatus::DownstreamClosed |
			ActorExitStatus::Killed |
			ActorExitStatus::Failure(_) |
			ActorExitStatus::Panicked => return Ok(()),
			ActorExitStatus::Quit | ActorExitStatus::Success => {},
		}
		Ok(())
	}
}

#[async_trait]
impl Handler<NewEventLock> for Indexer {
	type Reply = ();

	async fn handle(
		&mut self,
		message: NewEventLock,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		let NewEventLock(event_lock) = &message;
		self.event_lock = event_lock.clone();
		Ok(())
	}
}

#[async_trait]
impl Handler<ContextualTriples> for Indexer {
	type Reply = ();

	async fn handle(
		&mut self,
		message: ContextualTriples,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		let _indexing_items = message.event_payload();
		// items are document file vs triples
		// we would want to index the triples and send to various storages
		Err(ActorExitStatus::Success)
	}
}
