use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use common::{CollectedBytes, CollectionBatch, CollectionCounter, TerimateSignal};
use futures::StreamExt;
use sources::SourceError;
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
	pub event_receiver: Option<mpsc::Receiver<CollectedBytes>>,
	data_pollers: Vec<Arc<dyn sources::Source>>,
	workflow_handles: Vec<JoinHandle<Result<(), SourceError>>>,
	file_buffers: HashMap<String, Vec<CollectedBytes>>,
	file_size: HashMap<String, usize>,
	pub counters: CollectionCounter,
	// terminate signal to kill actors in the pipeline.
	pub terminate_sig: TerimateSignal,
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
			file_buffers: HashMap::new(),
			file_size: HashMap::new(),
			counters: CollectionCounter::new(),
			event_receiver: None,
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
			for handle in &self.workflow_handles {
				if handle.is_finished() {
					error!("Data Source is already finished");
					return Err(ActorExitStatus::Success);
				}
			}
			return Ok(());
		}

		info!("Starting data source collection for {}", self.id);
		let (event_sender, event_receiver) = mpsc::channel(1000);
		self.event_receiver = Some(event_receiver);
		for data_poller in &self.data_pollers {
			let data_poller = data_poller.clone();
			let event_sender = event_sender.clone();

			// Store the JoinHandle with the result in the Collector struct
			let handle = tokio::spawn(async move {
				let result = data_poller.poll_data().await;
				match result {
					Ok(mut stream) => {
						while let Some(data) = stream.next().await {
							match data {
								Ok(bytes) =>
									if let Err(e) = event_sender.send(bytes).await {
										error!("Failed to send data: {:?}", e);
									},

								Err(e) => {
									error!("Failed to poll data: {:?}", e);
									if let Err(e) = event_sender
										.send(CollectedBytes::new(
											None,
											None,
											false,
											None,
											None,
											"".to_string(),
										))
										.await
									{
										error!("Failed to send failure signal: {:?}", e);
									}
									return Err(e);
								},
							}
						}

						if let Err(e) = event_sender
							.send(CollectedBytes::new(None, None, true, None, None, "".to_string()))
							.await
						{
							error!("Failed to send EOF signal: {:?}", e);
						}
						Ok(())
					},
					Err(e) => {
						error!("Failed to poll data: {:?}", e);
						if let Err(e) = event_sender
							.send(CollectedBytes::new(
								None,
								None,
								false,
								None,
								None,
								"".to_string(),
							))
							.await
						{
							error!("Failed to send failure signal: {:?}", e);
						}
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
		let mut events_collected = HashMap::new();
		let mut counter = 0;
		let mut is_success = false;
		let mut is_failure = false;
		let event_receiver = self.event_receiver.as_mut().unwrap();

		loop {
			tokio::select! {
				event_opt = event_receiver.recv() => {
					if let Some(event_data) = event_opt {
						// If the payload is empty, skip the event
						if !event_data.eof && event_data.file.is_none() {
							is_failure = true;
							self.event_receiver.take();
							break;
						}

						if event_data.eof && event_data.file.is_none() {
							is_success = true;
							self.event_receiver.take();
							break;
						}
						let size = event_data.size.unwrap_or_default();

						// Update file size in self.file_size
						if let Some(file_path) = event_data.file.clone() {
							let file_path_str = file_path.to_string_lossy().to_string();
							if let Some(file_size) = self.file_size.get_mut(&file_path_str) {
								*file_size += size;
							} else {
								self.file_size.insert(file_path_str.clone(), size);
							}
						}

						let file_path = event_data.file.clone().unwrap_or_default();
						let file_path_str = file_path.to_string_lossy().to_string();
						if event_data.eof {
							if let Some(buffer) = self.file_buffers.remove(&file_path_str) {
								self.counters.increment_total_docs();
								self.counters.increment_ext_counter(&event_data.extension.clone().unwrap_or_default());
								events_collected.insert(file_path_str, buffer);
							}
						} else {
							self.file_buffers.entry(file_path_str.clone()).or_default().push(event_data);
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
				let events_batch = CollectionBatch::new(
					file,
					&chunks[0].extension.clone().unwrap_or_default(),
					chunks.clone(),
				);
				let batches_error = ctx.send_message(event_streamer_messagebus, events_batch).await;
				if batches_error.is_err() {
					error!("Failed to send bytes batch: {:?}", batches_error);
				}
				self.file_buffers.remove(file);
			}
		}
		events_collected.clear();
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
		for handle in self.workflow_handles.iter() {
			handle.abort();
		}
		self.workflow_handles.clear();
		Ok(())
	}
}
