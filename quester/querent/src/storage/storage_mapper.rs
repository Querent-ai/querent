use super::{ContextualEmbeddings, ContextualTriples};
use crate::{EventLock, NewEventLock};
use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{RuntimeType, SemanticKnowledgePayload, StorageMapperCounters, VectorPayload};
use querent_synapse::callbacks::EventType;
use std::{collections::HashMap, sync::Arc};
use storage::Storage;
use tokio::runtime::Handle;

pub struct StorageMapper {
	qflow_id: String,
	timestamp: u64,
	counters: Arc<StorageMapperCounters>,
	publish_event_lock: EventLock,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
}

impl StorageMapper {
	pub fn new(
		qflow_id: String,
		timestamp: u64,
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	) -> Self {
		Self {
			qflow_id,
			timestamp,
			counters: Arc::new(StorageMapperCounters::new()),
			publish_event_lock: EventLock::default(),
			event_storages,
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

	pub fn get_qflow_id(&self) -> String {
		self.qflow_id.clone()
	}

	pub fn get_publish_event_lock(&self) -> EventLock {
		self.publish_event_lock.clone()
	}
}

#[async_trait]
impl Actor for StorageMapper {
	type ObservableState = Arc<StorageMapperCounters>;

	fn observable_state(&self) -> Self::ObservableState {
		self.counters.clone()
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
			ActorExitStatus::Quit | ActorExitStatus::Success => {
				log::info!("StorageMapper exiting with success");
			},
		}
		Ok(())
	}
}
#[async_trait]
impl Handler<ContextualTriples> for StorageMapper {
	type Reply = ();

	async fn handle(
		&mut self,
		message: ContextualTriples,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.counters.increment_total(message.len() as u64);
		self.counters.increment_event_count(message.event_type(), message.len() as u64);
		let event_type = message.event_type();

		// Iterate over all storages in self.event_storages
		for (stored_event_type, storage) in &self.event_storages {
			if stored_event_type == &event_type {
				for storage in storage.iter() {
					let storage_clone = storage.clone();
					let storage_items = message.clone().event_payload();
					// Spawn a task for each storage insertion
					tokio::spawn(insert_graph_async(storage_clone, storage_items));
				}
			}
		}

		Ok(())
	}
}

#[async_trait]
impl Handler<ContextualEmbeddings> for StorageMapper {
	type Reply = ();

	async fn handle(
		&mut self,
		message: ContextualEmbeddings,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.counters.increment_total(message.len() as u64);
		self.counters.increment_event_count(message.event_type(), message.len() as u64);
		let event_type = message.event_type();
		let qflow_id = message.qflow_id();

		// Iterate over all storages in self.event_storages
		for (stored_event_type, storage) in &self.event_storages {
			if stored_event_type == &event_type {
				for storage in storage.iter() {
					let storage_clone = storage.clone();
					let qflow_id_clone = qflow_id.clone();
					let storage_items = message.clone().event_payload();

					// Spawn a task for each storage insertion
					tokio::spawn(insert_vector_async(storage_clone, qflow_id_clone, storage_items));
				}
			}
		}

		Ok(())
	}
}

async fn insert_graph_async(
	storage: Arc<dyn Storage>,
	storage_items: Vec<(String, SemanticKnowledgePayload)>,
) -> Result<(), ActorExitStatus> {
	let upsert_result = storage.insert_graph(&storage_items).await;
	match upsert_result {
		Ok(()) => {
			// Increment counters if insertion is successful
			// Note: Access to self.counters would require synchronization if used here
			Ok(())
		},
		Err(e) => {
			// Handle error if insertion fails
			log::error!("Error while inserting graphs: {:?}", e);
			// Depending on your error handling strategy, you might want to propagate the error
			// back to the caller or handle it differently
			Err(ActorExitStatus::Failure(e.source))
		},
	}
}

async fn insert_vector_async(
	storage: Arc<dyn Storage>,
	qflow_id: String,
	storage_items: Vec<(String, VectorPayload)>,
) -> Result<(), ActorExitStatus> {
	let upsert_result = storage.insert_vector(qflow_id, &storage_items).await;
	match upsert_result {
		Ok(()) => {
			// Increment counters if insertion is successful
			// Note: Access to self.counters would require synchronization if used here
			Ok(())
		},
		Err(e) => {
			// Handle error if insertion fails
			log::error!("Error while inserting vector: {:?}", e);
			// Depending on your error handling strategy, you might want to propagate the error
			// back to the caller or handle it differently
			Err(ActorExitStatus::Failure(e.source))
		},
	}
}

#[async_trait]
impl Handler<NewEventLock> for StorageMapper {
	type Reply = ();

	async fn handle(
		&mut self,
		message: NewEventLock,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		let NewEventLock(publish_event_lock) = &message;
		self.publish_event_lock = publish_event_lock.clone();
		Ok(())
	}
}
