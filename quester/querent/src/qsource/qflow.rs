use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use log;
use querent_synapse::{
	callbacks::{interface::EventHandler, EventState, EventType},
	config::{config::WorkflowConfig, Config},
	querent::{Querent, QuerentError, Workflow, WorkflowBuilder},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
	sync::mpsc,
	task::JoinHandle,
	time::{self},
};

use crate::{
	EventLock, EventStreamer, EventsBatch, EventsCounter, NewEventLock, Source, SourceContext,
	BATCH_NUM_EVENTS_LIMIT, EMIT_BATCHES_TIMEOUT,
};

pub struct Qflow {
	pub id: String,
	pub workflow: Workflow,
	pub event_lock: EventLock,
	pub counters: Arc<EventsCounter>,
	event_sender: mpsc::Sender<(EventType, EventState)>,
	event_receiver: mpsc::Receiver<(EventType, EventState)>,
	workflow_handle: Option<JoinHandle<Result<(), QuerentError>>>,
}

impl Qflow {
	pub fn new(id: String, workflow: Workflow) -> Self {
		let (event_sender, event_receiver) = mpsc::channel(10000);
		let workflow_event_handler = EventHandler::new(Some(event_sender.clone()));
		let mut config_copy = workflow.config.unwrap_or(Config::default());
		let workflow_config = WorkflowConfig {
			name: config_copy.workflow.name,
			id: config_copy.workflow.id,
			config: config_copy.workflow.config,
			inner_channel: None,
			channel: None,
			inner_event_handler: Some(workflow_event_handler),
			event_handler: None,
		};
		config_copy.workflow = workflow_config;

		let workflow = WorkflowBuilder::new(workflow.id.as_str())
			.name(workflow.name.as_str())
			.import(Some(workflow.import))
			.attr(Some(workflow.attr))
			.code(workflow.code)
			.config(config_copy)
			.arguments(workflow.arguments)
			.build();
		Self {
			id: id.clone(),
			workflow,
			event_lock: EventLock::default(),
			counters: Arc::new(EventsCounter::new(id.clone())),
			event_sender,
			event_receiver,
			workflow_handle: None,
		}
	}

	pub fn get_config(&self) -> Option<Config> {
		self.workflow.config.clone()
	}
}

#[async_trait]
impl Source for Qflow {
	async fn initialize(
		&mut self,
		event_streamer_messagebus: &MessageBus<EventStreamer>,
		ctx: &SourceContext,
	) -> Result<(), ActorExitStatus> {
		let querent = Querent::new().map_err(|e| {
			ActorExitStatus::Failure(
				anyhow::anyhow!("Failed to initialize querent: {:?}", e).into(),
			)
		})?;
		querent.add_workflow(self.workflow.clone()).map_err(|e| {
			ActorExitStatus::Failure(anyhow::anyhow!("Failed to add workflow: {:?}", e).into())
		})?;

		let workflow_id = self.workflow.id.clone();
		let event_sender = self.event_sender.clone();
		// Store the JoinHandle with the result in the Qflow struct
		self.workflow_handle = Some(tokio::spawn(async move {
			let result = querent.start_workflows().await;
			match result {
				Ok(()) => {
					// Handle the success
					log::info!("Successfully the workflow with id: {}", workflow_id);
					// send yourself a success message to stop
					event_sender
						.send((
							EventType::Success,
							EventState {
								event_type: EventType::Success,
								timestamp: chrono::Utc::now().timestamp_millis() as f64,
								payload: "".to_string(),
								file: "".to_string(),
							},
						))
						.await
						.unwrap();
					Ok(())
				},
				Err(err) => {
					// Handle the error, e.g., log it
					log::error!("Failed to start the workflow with id: {}", workflow_id);
					event_sender
						.send((
							EventType::Failure,
							EventState {
								event_type: EventType::Failure,
								timestamp: chrono::Utc::now().timestamp_millis() as f64,
								payload: format!(
									"workflow with id: {:?} failed with error: {:?}",
									workflow_id, err
								),
								file: "".to_string(),
							},
						))
						.await
						.unwrap();
					Err(err)
				},
			}
		}));
		let event_lock = self.event_lock.clone();
		ctx.send_message(event_streamer_messagebus, NewEventLock(event_lock)).await?;
		Ok(())
	}

	async fn emit_events(
		&mut self,
		event_streamer_messagebus: &MessageBus<EventStreamer>,
		ctx: &SourceContext,
	) -> Result<Duration, ActorExitStatus> {
		let deadline = time::sleep(EMIT_BATCHES_TIMEOUT);
		tokio::pin!(deadline);
		let mut events_collected = HashMap::new();
		let mut is_success = false;
		let mut is_failure = false;
		let mut counter = 0;
		loop {
			tokio::select! {
				event_opt = self.event_receiver.recv() => {
					if let Some((event_type, event_data)) = event_opt {
						if event_type == EventType::Success {
							is_success = true;
							break;
						}
						if event_type == EventType::Failure {
							is_failure = true;
							break;
						}
						self.counters.increment_total();
						events_collected.insert(event_type, event_data);
						counter += 1;
					}
					if counter >= BATCH_NUM_EVENTS_LIMIT {
						self.counters.increment_processed(counter as u64);
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
			let events_batch = EventsBatch::new(
				self.id.clone(),
				events_collected,
				chrono::Utc::now().timestamp_millis() as u64,
			);
			ctx.send_message(event_streamer_messagebus, events_batch).await?;
		}
		if is_success {
			ctx.protect_future(self.event_lock.kill()).await;
			ctx.send_exit_with_success(event_streamer_messagebus).await?;
			return Err(ActorExitStatus::Success);
		}
		if is_failure {
			ctx.protect_future(self.event_lock.kill()).await;
			return Err(ActorExitStatus::Failure(
				anyhow::anyhow!("Querent Python workflow failed").into(),
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
}
