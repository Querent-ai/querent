use std::{collections::HashMap, sync::Arc};

use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{IndexerCounters, RuntimeType, SemanticKnowledgePayload};
use storage::Storage;
use tokio::runtime::Handle;
use tracing::error;

use crate::{ContextualTriples, EventLock, IndexerKnowledge, NewEventLock};

pub struct Indexer {
	pub qflow_id: String,
	pub timestamp: u64,
	pub counters: Arc<IndexerCounters>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub event_lock: EventLock,
}

impl Indexer {
	pub fn new(qflow_id: String, timestamp: u64, index_storages: Vec<Arc<dyn Storage>>) -> Self {
		Self {
			qflow_id,
			timestamp,
			counters: Arc::new(IndexerCounters::new()),
			event_lock: EventLock::default(),
			index_storages,
		}
	}

	pub fn get_counters(&self) -> Arc<IndexerCounters> {
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
	type ObservableState = Arc<IndexerCounters>;

	fn observable_state(&self) -> Self::ObservableState {
		self.counters.clone()
	}

	fn name(&self) -> String {
		"Indexer".to_string()
	}

	fn queue_capacity(&self) -> QueueCapacity {
		QueueCapacity::Bounded(5)
	}

	fn runtime_handle(&self) -> Handle {
		RuntimeType::NonBlocking.get_runtime_handle()
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
			ActorExitStatus::Quit | ActorExitStatus::Success => {
				log::info!("Indexer exiting with success");
			},
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
		error!("indexing_items: {:?}", _indexing_items);
		// items are document file vs triples
		// we would want to index the triples and send to various storages
		Err(ActorExitStatus::Success)
	}
}

#[async_trait]
impl Handler<IndexerKnowledge> for Indexer {
	type Reply = ();

	async fn handle(
		&mut self,
		_message: IndexerKnowledge,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		let knowledge = _message.triples;
		for storage in &self.index_storages {
			storage.index_knowledge(self.qflow_id.clone(), &knowledge).await.map_err(|e| {
				log::error!("Error indexing knowledge: {:?}", e);
				ActorExitStatus::Failure(anyhow::anyhow!("Failed to index: {:?}", e).into())
			})?;
		}
		// collect statistics
		let mut doc_map: HashMap<String, Vec<SemanticKnowledgePayload>> = HashMap::new();
		for (doc, _source, _image_id, payload) in knowledge {
			doc_map.entry(doc.clone()).or_insert_with(Vec::new).push(payload);
		}
		doc_map.iter().for_each(|(_doc, triples)| {
			self.counters.increment_total_sentences_indexed(triples.len() as u64);

			let (s, p, o) = triples.iter().fold((0, 0, 0), |(s_acc, p_acc, o_acc), payload| {
				let s = if !payload.subject.is_empty() { s_acc + 1 } else { s_acc };
				let p = if !payload.predicate.is_empty() { p_acc + 1 } else { p_acc };
				let o = if !payload.object.is_empty() { o_acc + 1 } else { o_acc };

				(s, p, o)
			});

			self.counters.increment_total_subjects_indexed(s as u64);
			self.counters.increment_total_predicates_indexed(p as u64);
			self.counters.increment_total_objects_indexed(o as u64);
		});
		Ok(())
	}
}
