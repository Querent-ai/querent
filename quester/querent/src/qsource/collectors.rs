use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use common::{CollectedBytes, CollectionBatch, CollectionCounter, TerimateSignal};
use querent_synapse::querent::QuerentError;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::mpsc, task::JoinHandle, time};
use tracing::{error, info};

use crate::{
	EventLock, EventStreamer, NewEventLock, Source, SourceContext, BATCH_NUM_EVENTS_LIMIT,
	EMIT_BATCHES_TIMEOUT,
};

pub struct Collector {
	pub id: String,
	pub event_lock: EventLock,
	data_poller: Arc<dyn sources::Source>,
	event_sender: mpsc::Sender<CollectedBytes>,
	event_receiver: mpsc::Receiver<CollectedBytes>,
	workflow_handle: Option<JoinHandle<Result<(), QuerentError>>>,
	file_buffers: HashMap<String, Vec<CollectedBytes>>,
	file_size: HashMap<String, usize>,
	pub counters: CollectionCounter,
	// terimatesignal to kill actors in the pipeline.
	pub terminate_sig: TerimateSignal,
}

impl Collector {
	pub fn new(
		id: String,
		data_poller: Arc<dyn sources::Source>,
		event_sender: mpsc::Sender<CollectedBytes>,
		event_receiver: mpsc::Receiver<CollectedBytes>,
		terminate_sig: TerimateSignal,
	) -> Self {
		Self {
			id: id.clone(),
			event_lock: EventLock::default(),
			event_sender,
			event_receiver,
			workflow_handle: None,
			terminate_sig,
			data_poller,
			file_buffers: HashMap::new(),
			file_size: HashMap::new(),
			counters: CollectionCounter::new(),
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
		if self.workflow_handle.is_some() {
			if self.workflow_handle.as_ref().unwrap().is_finished() {
				error!("Data Source is already finished");
				return Err(ActorExitStatus::Success);
			}
			return Ok(());
		}

		info!("Starting data source collection for {}", self.id);
		let event_sender = self.event_sender.clone();
		let data_poller = self.data_poller.clone();

		// Store the JoinHandle with the result in the Collector struct
		self.workflow_handle = Some(tokio::spawn(async move {
			let result = { data_poller.poll_data(event_sender.clone()).await };
			match result {
				Ok(_) => {
					event_sender
						.send(CollectedBytes::new(None, None, true, None, None))
						.await
						.unwrap();
					Ok(())
				},
				Err(e) => {
					error!("Failed to poll data: {:?}", e);
					let err = Err(QuerentError::internal(format!("Failed to poll data: {:?}", e)));
					event_sender
						.send(CollectedBytes::new(None, None, false, None, None))
						.await
						.unwrap();
					err
				},
			}
		}));

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
		if self.workflow_handle.is_none() {
			return Err(ActorExitStatus::Success);
		}
		let deadline = time::sleep(EMIT_BATCHES_TIMEOUT);
		tokio::pin!(deadline);
		let mut events_collected = HashMap::new();
		let mut counter = 0;
		let mut is_success = false;
		let mut is_failure = false;
		loop {
			tokio::select! {
				event_opt = self.event_receiver.recv() => {
					if let Some(event_data) = event_opt {
						// If the payload is empty, skip the event
						if !event_data.eof && event_data.file.is_none() {
							is_failure = true;
							break;
						}

						if event_data.eof && event_data.file.is_none() {
							is_success = true;
							break;
						}
						let size = event_data.size.unwrap_or_default();

						// Update file size in self.file_size
						if let Some(file_path) = event_data.file.clone() {
							let file_path_str = file_path.to_string_lossy().to_string();
							if let Some(file_size) = self.file_size.get_mut(file_path_str.as_str()) {
								*file_size += size;
							} else {
								self.file_size.insert(file_path_str.clone(), size);
							}
						}


						let file_path = event_data.file.clone().unwrap_or_default();
						let file_path_str = file_path.to_string_lossy().to_string();
						if event_data.eof {
							if let Some(buffer) = self.file_buffers.remove(file_path_str.clone().as_str()) {
								self.counters.increment_total_docs();
								self.counters.increment_ext_counter(&event_data.extension.clone().unwrap_or_default());
								events_collected.insert(file_path_str, buffer);
							}
						} else {
							if let Some(buffer) = self.file_buffers.get_mut(file_path_str.as_str()) {
								buffer.push(event_data);
							} else {
								let mut buffer = Vec::new();
								buffer.push(event_data);
								self.file_buffers.insert(file_path_str.clone(), buffer);
							}
						}
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
		if !events_collected.is_empty() {
			for (file, chunks) in events_collected.iter() {
				if chunks.is_empty() {
					continue;
				}
				let total_size = self.file_size.get(file).unwrap_or(&0);
				self.counters.increment_total_bytes(total_size.clone() as u64);
				let events_batch = CollectionBatch::new(
					file,
					&chunks[0].clone().extension.unwrap_or_default(),
					chunks,
				);
				let batches_error =
					ctx.send_message(event_streamer_messagebus, events_batch.clone()).await;
				if batches_error.is_err() {
					error!("Failed to send bytes batch: {:?}", batches_error);
					// Re-trying
					let retry_error =
						ctx.send_message(event_streamer_messagebus, events_batch).await;
					if retry_error.is_err() {
						return Err(ActorExitStatus::Failure(
							anyhow::anyhow!("Failed to send bytes batch: {:?}", retry_error).into(),
						));
					}
				}
			}
		}
		if is_success {
			return Err(ActorExitStatus::Success);
		}
		if is_failure {
			return Err(ActorExitStatus::Failure(
				anyhow::anyhow!("Collector failed: {:?}", self.id).into(),
			));
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
		match self.workflow_handle.take() {
			Some(handle) => {
				handle.abort();
			},
			None => {
				info!("Collector is already finished");
			},
		}
		Ok(())
	}
}
