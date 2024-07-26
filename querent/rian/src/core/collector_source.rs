use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use common::{CollectedBytes, CollectionBatch, CollectionCounter, TerimateSignal};
use futures::StreamExt;
use sources::SourceError;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, task::JoinHandle, time};
use tracing::{error, info};

use crate::{
	EventLock, EventStreamer, NewEventLock, Source, SourceContext, BATCH_NUM_EVENTS_LIMIT,
	EMIT_BATCHES_TIMEOUT, NUMBER_FILES_IN_MEMORY,
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
}

impl Collector {
	pub fn new(
		id: String,
		data_pollers: Vec<Arc<dyn sources::Source>>,
		terminate_sig: TerimateSignal,
	) -> Self {
		Self {
			id: id.clone(),
			event_lock: EventLock::default(),
			workflow_handles: Vec::new(),
			terminate_sig,
			data_pollers,
			counters: CollectionCounter::new(),
			event_receiver: None,
			leftover_collection_batches: Vec::new(),
		}
	}
}

#[async_trait]
impl Source for Collector {
	async fn initialize(
		&mut self,
		event_streamer_messagebus: &MessageBus<EventStreamer>,
		ctx: &SourceContext,
	) -> Result<(), ActorExitStatus> {
		if !self.workflow_handles.is_empty() {
			self.workflow_handles.retain(|handle| !handle.is_finished());
			if self.workflow_handles.is_empty() {
				return Err(ActorExitStatus::Success);
			}
			for handle in &self.workflow_handles {
				if handle.is_finished() {
					error!("Data Source is already finished");
					return Err(ActorExitStatus::Success);
				}
			}
			return Ok(());
		}
		info!("Starting data source collection for {}", self.id);
		let (event_sender, event_receiver) = mpsc::channel(10);
		self.event_receiver = Some(event_receiver);
		for data_poller in &self.data_pollers {
			let data_poller = data_poller.clone();
			let event_sender = event_sender.clone();
			let terminate_sig = self.terminate_sig.clone();

			let handle = tokio::spawn(async move {
				let result = data_poller.poll_data().await;
				match result {
					Ok(mut stream) => {
						while terminate_sig.is_alive() {
							let mut buffer_data: Vec<CollectedBytes> = Vec::new();
							let mut file = String::new();
							let mut extension = String::new();
							// loop over stream till you get a eof, when None is received, break the loop
							while let Some(Ok(data)) = stream.next().await {
								if data.eof {
									file = data
										.file
										.clone()
										.unwrap_or_default()
										.to_string_lossy()
										.to_string();
									extension = data.extension.clone().unwrap_or_default();
									break;
								}
								buffer_data.push(data);
							}

							// we have no data to send
							if buffer_data.is_empty() {
								error!("No data to send");
								return Ok(());
							}
							error!("Sending data to event sender");
							let batch = CollectionBatch::new(&file, &extension, buffer_data);
							if let Err(e) = event_sender.send(batch).await {
								error!("Failed to send EOF signal: {:?}", e);
							}
						}
						Ok(())
					},
					Err(e) => {
						error!("Failed to poll data: {:?}", e);
						Err(e)
					},
				}
			});
			self.workflow_handles.push(handle);
		}

		info!("Started the collector for {}", self.id);
		let event_lock = self.event_lock.clone();
		ctx.send_message(event_streamer_messagebus, NewEventLock(event_lock)).await?;
		Ok(())
	}

	async fn emit_events(
		&mut self,
		event_streamer_messagebus: &MessageBus<EventStreamer>,
		ctx: &SourceContext,
	) -> Result<Duration, ActorExitStatus> {
		if self.workflow_handles.is_empty() {
			return Err(ActorExitStatus::Success);
		}
		let deadline = time::sleep(EMIT_BATCHES_TIMEOUT);
		tokio::pin!(deadline);
		let mut counter = 0;
		let event_receiver = self.event_receiver.as_mut().unwrap();
		let mut files = Vec::new();
		if self.leftover_collection_batches.is_empty() {
			loop {
				tokio::select! {
					event_opt = event_receiver.recv() => {
						if let Some(event_data) = event_opt {
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
		error!("Count of files: {:?}", files.len());
		if files.len() > NUMBER_FILES_IN_MEMORY {
			error!("Number of files in memory: {:?}", files.len());
		}
		if !files.is_empty() {
			for batch in files {
				let batches_error = event_streamer_messagebus.ask(batch).await;
				if batches_error.is_err() {
					return Err(ActorExitStatus::Failure(
						anyhow::anyhow!("Failed to send batch: {:?}", batches_error).into(),
					));
				}
				let batches_error = batches_error.unwrap();
				match batches_error {
					Ok(batch) => match batch {
						Some(batch) => {
							error!("EventStreamer returned a batch");
							self.leftover_collection_batches.push(batch);
						},
						None => {},
					},
					Err(e) => {
						error!("Error sending message to StorageMapper: {:?}", e);
						return Err(ActorExitStatus::Failure(
							anyhow::anyhow!("Failed to send batch: {:?}", e).into(),
						));
					},
				}
			}
		}
		if self.leftover_collection_batches.is_empty() {
			return Ok(Duration::from_secs(1));
		}
		Ok(Duration::default())
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
		self.terminate_sig.kill();
		Ok(())
	}
}
