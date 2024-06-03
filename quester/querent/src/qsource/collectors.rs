use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use common::{CollectedBytes, CollectionBatch, TerimateSignal};
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
				Ok(_) => Ok(()),
				Err(e) => {
					error!("Failed to poll data: {:?}", e);
					Err(QuerentError::internal(format!("Failed to poll data: {:?}", e)))
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
						let file_path = event_data.file.clone().unwrap_or_default();
						let file_path_str = file_path.to_string_lossy().to_string();
						if event_data.eof {
							if let Some(buffer) = self.file_buffers.remove(file_path_str.clone().as_str()) {
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
					&chunks[0].clone().extension.unwrap_or_default(),
					chunks,
				);
				let batches_error =
					ctx.send_message(event_streamer_messagebus, events_batch.clone()).await;
				if batches_error.is_err() {
					error!("Failed to send events batch: {:?}", batches_error);
					// Re-trying
					let retry_error =
						ctx.send_message(event_streamer_messagebus, events_batch).await;
					if retry_error.is_err() {
						return Err(ActorExitStatus::Failure(
							anyhow::anyhow!("Failed to send events batch: {:?}", retry_error)
								.into(),
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
		// Implement observable state if needed
		serde_json::Value::Null
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
				info!("QSource is already finished");
			},
		}
		Ok(())
	}
}
