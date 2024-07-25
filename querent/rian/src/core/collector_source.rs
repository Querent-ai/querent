use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use common::{CollectedBytes, CollectionBatch, CollectionCounter, TerimateSignal};
use futures::StreamExt;
use sources::SourceError;
use std::{
	collections::{HashMap, HashSet},
	sync::{atomic::AtomicI32, Arc},
	time::Duration,
};
use tokio::{sync::mpsc, task::JoinHandle, time};
use tracing::{error, info};

use crate::{
	EventLock, EventStreamer, NewEventLock, Source, SourceContext, BATCH_NUM_EVENTS_LIMIT,
	EMIT_BATCHES_TIMEOUT, NUMBER_FILES_IN_MEMORY,
};

pub struct Collector {
	pub id: String,
	pub event_lock: EventLock,
	pub is_set_once: bool,
	pub event_receiver: Option<mpsc::Receiver<CollectedBytes>>,
	data_pollers: Vec<Arc<dyn sources::Source>>,
	workflow_handles: Vec<JoinHandle<Result<(), SourceError>>>,
	file_buffers: HashMap<String, Vec<CollectedBytes>>,
	file_size: HashMap<String, usize>,
	availble_files: HashSet<String>,
	pub counters: CollectionCounter,
	// terminate signal to kill actors in the pipeline.
	pub terminate_sig: TerimateSignal,
	// internal counter for exit when data pollers are finished
	poller_finished_counter: AtomicI32,
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
			poller_finished_counter: AtomicI32::new(0),
			availble_files: HashSet::new(),
			is_set_once: false,
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
		if self.is_set_once {
			return Err(ActorExitStatus::Success);
		}
		info!("Starting data source collection for {}", self.id);
		let (event_sender, event_receiver) = mpsc::channel(10);
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
		self.is_set_once = true;
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
		let mut is_failure = false;
		let event_receiver = self.event_receiver.as_mut().unwrap();
		if self.availble_files.len() == 0 {
			loop {
				tokio::select! {
					event_opt = event_receiver.recv() => {
						if let Some(event_data) = event_opt {
							if event_data.file.is_none() && event_data.data.is_none() {
								is_failure = true;
								break;
							}
							let size = event_data.size.unwrap_or_default();
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
									self.counters.increment_total_docs(1);
									self.availble_files.insert(file_path_str.clone());
									self.counters.increment_ext_counter(&event_data.extension.clone().unwrap_or_default());
									// only keep the last NUMBER_FILES_IN_MEMORY files in memory
									if self.availble_files.len() >= NUMBER_FILES_IN_MEMORY {
										break;
									}
							} else {
								self.file_buffers.entry(file_path_str).or_default().push(event_data);
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
		}
		if !self.availble_files.is_empty() {
			let mut file_buffer_to_remove = Vec::new();
			for available_file_from_buffer in self.availble_files.iter() {
				let chunks =
					self.file_buffers.remove(available_file_from_buffer).unwrap_or_default();
				if chunks.is_empty() {
					continue;
				}
				let events_batch = CollectionBatch::new(
					available_file_from_buffer,
					&chunks[0].extension.clone().unwrap_or_default(),
					chunks,
				);
				let batches_error = event_streamer_messagebus.ask(events_batch).await;
				if batches_error.is_err() {
					return Err(ActorExitStatus::Failure(
						anyhow::anyhow!("Failed to send batch: {:?}", batches_error).into(),
					));
				}
				let batches_error = batches_error.unwrap();
				match batches_error {
					Ok(batch) => match batch {
						Some(batch) => {
							self.file_buffers
								.entry(batch.file.to_string())
								.or_default()
								.extend(batch.bytes);
							return Ok(Duration::from_secs(1));
						},
						None => {
							file_buffer_to_remove.push(available_file_from_buffer.clone());
						},
					},
					Err(e) => {
						error!("Error sending message to StorageMapper: {:?}", e);
						return Err(ActorExitStatus::Failure(
							anyhow::anyhow!("Failed to send batch: {:?}", e).into(),
						));
					},
				}
			}

			for file in file_buffer_to_remove {
				self.availble_files.remove(&file);
			}
		}

		if self.data_pollers.len() as usize ==
			self.poller_finished_counter.load(std::sync::atomic::Ordering::SeqCst) as usize &&
			self.availble_files.is_empty() &&
			self.file_buffers.is_empty()
		{
			return Err(ActorExitStatus::Success);
		}
		if is_failure &&
			self.data_pollers.len() as usize ==
				self.poller_finished_counter.load(std::sync::atomic::Ordering::SeqCst)
					as usize &&
			self.availble_files.is_empty() &&
			self.file_buffers.is_empty()
		{
			return Err(ActorExitStatus::Failure(
				anyhow::anyhow!("Collector failed: {:?}", self.id).into(),
			));
		}
		Ok(Duration::from_secs(1))
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
