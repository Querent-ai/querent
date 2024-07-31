use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use common::{CollectedBytes, CollectionBatch, CollectionCounter, TerimateSignal};
use futures::StreamExt;
use sources::{SourceError, SourceErrorKind};
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, task::JoinHandle, time};
use tracing::{debug, error, info};

use crate::{
	ingest::ingestor_service::IngestorService, EventLock, EventStreamer, Source, SourceContext,
	BATCH_NUM_EVENTS_LIMIT, EMIT_BATCHES_TIMEOUT, NUMBER_FILES_IN_MEMORY,
};

pub struct Collector {
	pub id: String,
	pub event_lock: EventLock,
	pub event_receiver: Option<mpsc::Receiver<CollectionBatch>>,
	data_pollers: Vec<Arc<dyn sources::Source>>,
	workflow_handles: Vec<JoinHandle<Result<(), SourceError>>>,
	pub counters: CollectionCounter,
	pub terminate_sig: TerimateSignal,
	leftover_collection_batches: Vec<CollectionBatch>,
	semaphore: Arc<tokio::sync::Semaphore>,
	source_counter_semaphore: Arc<tokio::sync::Semaphore>,
}

impl Collector {
	pub fn new(
		id: String,
		data_pollers: Vec<Arc<dyn sources::Source>>,
		terminate_sig: TerimateSignal,
	) -> Self {
		let total_pollers = data_pollers.len();
		Self {
			id: id.clone(),
			event_lock: EventLock::default(),
			workflow_handles: Vec::new(),
			terminate_sig,
			data_pollers,
			counters: CollectionCounter::new(),
			event_receiver: None,
			leftover_collection_batches: Vec::new(),
			source_counter_semaphore: Arc::new(tokio::sync::Semaphore::new(total_pollers)),
			semaphore: Arc::new(tokio::sync::Semaphore::new(NUMBER_FILES_IN_MEMORY)),
		}
	}
}

#[async_trait]
impl Source for Collector {
	async fn initialize(
		&mut self,
		_event_streamer_messagebus: &MessageBus<EventStreamer>,
		_ingestor_messagebus: &MessageBus<IngestorService>,
		ctx: &SourceContext,
	) -> Result<(), ActorExitStatus> {
		if self.source_counter_semaphore.available_permits() < self.data_pollers.len() {
			if self.source_counter_semaphore.available_permits() == 0 &&
				self.semaphore.available_permits() == 0
			{
				return Err(ActorExitStatus::Success);
			}
			return Ok(());
		}

		info!("Starting data source collection for {}", self.id);
		let (event_sender, event_receiver) = mpsc::channel(NUMBER_FILES_IN_MEMORY);
		self.event_receiver = Some(event_receiver);
		for data_poller in &self.data_pollers {
			let permit = self.source_counter_semaphore.clone().acquire_owned().await;
			let data_poller = data_poller.clone();
			let event_sender = event_sender.clone();
			let terminate_sig = self.terminate_sig.clone();
			let handle = tokio::spawn(async move {
				let _permit = permit.unwrap();
				let result = data_poller.poll_data().await;
				match result {
					Ok(mut stream) => {
						let mut buffer_data: Vec<CollectedBytes> = Vec::new();
						while let Some(Ok(data)) = stream.next().await {
							if terminate_sig.is_dead() {
								break;
							}
							let extension = data.extension.clone().unwrap_or_default();
							let file =
								data.file.clone().unwrap_or_default().to_string_lossy().to_string();
							let eof = data.eof;
							buffer_data.push(data);
							if eof {
								let batch =
									CollectionBatch::new(&file, &extension, buffer_data, None);
								if let Err(e) = event_sender.send(batch).await {
									error!("Failed to send data to event sender: {:?}", e);
								}
								buffer_data = Vec::new();
							}
						}
					},
					Err(e) => {
						error!("Failed to poll data: {:?}", e);
						return Err(e);
					},
				}

				let finished_batch =
					CollectionBatch::new(&"".to_string(), &"".to_string(), Vec::new(), None);
				if let Err(e) = event_sender.send(finished_batch).await {
					error!("Failed to send data to event sender: {:?}", e);
					return Err(SourceError::new(
						SourceErrorKind::Io,
						Arc::new(
							anyhow::anyhow!("Failed to send data to event sender: {:?}", e).into(),
						),
					));
				}

				Ok(())
			});
			self.workflow_handles.push(handle);
			ctx.record_progress();
		}
		info!("Started the collector for {}", self.id);
		Ok(())
	}

	async fn emit_events(
		&mut self,
		_event_streamer_messagebus: &MessageBus<EventStreamer>,
		ingestor_messagebus: &MessageBus<IngestorService>,
		ctx: &SourceContext,
	) -> Result<Duration, ActorExitStatus> {
		let mut collector_exit = false;
		let deadline = time::sleep(EMIT_BATCHES_TIMEOUT);
		tokio::pin!(deadline);
		let mut counter = 0;
		let event_receiver = self.event_receiver.as_mut().unwrap();
		let mut files = self.leftover_collection_batches.drain(..).collect::<Vec<_>>();
		if files.is_empty() {
			loop {
				tokio::select! {
					event_opt = event_receiver.recv() => {
						if let Some(event_data) = event_opt {
							if event_data.file.is_empty()
								&& event_data.ext.is_empty()
								&& event_data.bytes.is_empty() {
								// clean up the workflow handles
								self.workflow_handles.retain(|handle| !handle.is_finished());
								collector_exit = true;
								break;
							}
							self.counters.increment_total_docs(1);
							self.counters.increment_ext_counter(&event_data.ext.clone());
							files.push(event_data);
							counter += 1;
						}
						if counter >= BATCH_NUM_EVENTS_LIMIT {
							break;
						}
						ctx.record_progress();
					}
					_ = &mut deadline => {
						break;
					}
				}
			}
		}

		if !files.is_empty() {
			for mut batch in files {
				let permit = self.semaphore.clone().acquire_owned().await;
				match permit {
					Ok(permit) => {
						batch._permit = Some(permit);
					},
					Err(e) => {
						self.leftover_collection_batches.push(batch);
						debug!("Failed to acquire permit: {:?}", e);
						continue;
					},
				}
				let batches_error = ingestor_messagebus.ask(batch).await;
				if batches_error.is_err() {
					return Err(ActorExitStatus::Failure(
						anyhow::anyhow!("Failed to send batch: {:?}", batches_error).into(),
					));
				}
				let batches_error = batches_error.unwrap();
				match batches_error {
					Ok(batch) => match batch {
						Some(batch) => {
							error!("Ingestor returned a batch");
							self.leftover_collection_batches.push(batch);
						},
						None => {},
					},
					Err(e) => {
						error!("Error sending message to Ingestor: {:?}", e);
						return Err(ActorExitStatus::Failure(
							anyhow::anyhow!("Failed to send batch: {:?}", e).into(),
						));
					},
				}
			}
		}

		if collector_exit == true {
			return Err(ActorExitStatus::Success);
		}
		ctx.record_progress();
		return Ok(Duration::default());
	}

	fn name(&self) -> String {
		self.id.clone()
	}

	fn observable_state(&self) -> serde_json::Value {
		serde_json::to_value(&self.counters).unwrap()
	}

	async fn finalize(
		&mut self,
		_exit_status: &ActorExitStatus,
		_ctx: &SourceContext,
	) -> anyhow::Result<()> {
		for handle in self.workflow_handles.iter() {
			handle.abort();
		}
		self.workflow_handles.clear();
		Ok(())
	}
}
