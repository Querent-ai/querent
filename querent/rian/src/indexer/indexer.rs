use std::{collections::HashMap, sync::Arc};

use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{IndexerCounters, RuntimeType, SemanticKnowledgePayload};
use storage::Storage;
use tokio::runtime::Handle;

use crate::{EventLock, IndexerKnowledge, NewEventLock};

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
		QueueCapacity::Bounded(1)
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
impl Handler<IndexerKnowledge> for Indexer {
	type Reply = ();

	async fn handle(
		&mut self,
		_message: IndexerKnowledge,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		let knowledge = _message.triples;
		let mut doc_map: HashMap<String, Vec<SemanticKnowledgePayload>> = HashMap::new();
		for (doc, _source, _image_id, payload) in &knowledge {
			doc_map.entry(doc.clone()).or_insert_with(Vec::new).push(payload.clone());
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
		for storage in &self.index_storages {
			insert_index_async(self.qflow_id.clone(), storage.clone(), knowledge.clone())?;
		}
		Ok(())
	}
}

pub fn insert_index_async(
	collection_id: String,
	storage: Arc<dyn Storage>,
	storage_items: Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
) -> Result<(), ActorExitStatus> {
	tokio::spawn(async move {
		let upsert_result = storage.index_knowledge(collection_id, &storage_items).await;
		if let Err(e) = upsert_result {
			log::error!("Error inserting knowledge: {:?}", e);
		}
	});
	Ok(())
}
