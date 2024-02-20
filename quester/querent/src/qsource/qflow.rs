use actors::{ActorExitStatus, ActorHandle, MessageBus};
use async_trait::async_trait;
use common::{EventsBatch, EventsCounter, TerimateSignal};
use querent_synapse::{
	callbacks::{interface::EventHandler, EventState, EventType},
	comm::IngestedTokens,
	config::{config::WorkflowConfig, Config},
	querent::{Querent, QuerentError, Workflow, WorkflowBuilder},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
	sync::mpsc,
	task::JoinHandle,
	time::{self},
};
use tracing::{error, info};

use crate::{
	Collection, EventLock, EventStreamer, NewEventLock, Source, SourceContext,
	BATCH_NUM_EVENTS_LIMIT, EMIT_BATCHES_TIMEOUT,
};

pub struct Qflow {
	pub id: String,
	pub workflow: Workflow,
	pub event_lock: EventLock,
	pub counters: Arc<EventsCounter>,
	event_sender: mpsc::Sender<(EventType, EventState)>,
	event_receiver: mpsc::Receiver<(EventType, EventState)>,
	token_sender: Option<crossbeam_channel::Sender<IngestedTokens>>,
	workflow_handle: Option<JoinHandle<Result<(), QuerentError>>>,
	collection_handler: Option<ActorHandle<Collection>>,
	// terimatesignal to kill actors in the pipeline.
	pub terminate_sig: TerimateSignal,
}

impl Qflow {
	pub fn new(
		id: String,
		workflow: Workflow,
		token_sender: Option<crossbeam_channel::Sender<IngestedTokens>>,
		terminate_sig: TerimateSignal,
	) -> Self {
		let (event_sender, event_receiver) = mpsc::channel(1000);
		let workflow_event_handler: EventHandler = EventHandler::new(Some(event_sender.clone()));
		let mut config_copy = workflow.config.unwrap_or(Config::default());
		let workflow_config = WorkflowConfig {
			name: config_copy.workflow.name,
			id: config_copy.workflow.id,
			config: config_copy.workflow.config,
			inner_channel: config_copy.workflow.inner_channel,
			channel: None,
			inner_event_handler: Some(workflow_event_handler),
			event_handler: None,
			inner_tokens_feader: config_copy.workflow.inner_tokens_feader,
			tokens_feader: None,
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
			collection_handler: None,
			token_sender,
			terminate_sig,
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
		if self.workflow_handle.is_some() {
			if self.workflow_handle.as_ref().unwrap().is_finished() {
				error!("Qflow is already finished");
				return Err(ActorExitStatus::Success);
			}
			return Ok(());
		}
		// Start Collection Actor
		let collector = Collection::new(
			self.id.clone(),
			self.workflow.clone(),
			self.event_sender.clone(),
			self.token_sender.clone(),
		);

		info!("Starting the collector actor 📚");
		let (_collector_message_bus, collector_inbox) =
			ctx.spawn_actor().set_terminate_sig(self.terminate_sig.clone()).spawn(collector);
		self.collection_handler = Some(collector_inbox);

		info!("Starting the engine 🚀");
		let querent = Querent::new().map_err(|e| {
			ActorExitStatus::Failure(
				anyhow::anyhow!("Failed to initialize collector: {:?}", e).into(),
			)
		})?;

		querent.add_workflow(self.workflow.clone()).map_err(|e| {
			error!("Failed to add workflow: {:?}", e);
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
					log::info!("Successfully started the workflow with id: {}", workflow_id);
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
					log::error!(
						"Failed to run the workflow with id: {} and error: {:?}",
						workflow_id,
						err
					);
					event_sender
						.send((
							EventType::Failure,
							EventState {
								event_type: EventType::Failure,
								timestamp: chrono::Utc::now().timestamp_millis() as f64,
								payload: err.to_string(),
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
		if self.workflow_handle.is_none() {
			return Err(ActorExitStatus::Success);
		}
		let deadline = time::sleep(EMIT_BATCHES_TIMEOUT);
		tokio::pin!(deadline);
		let mut events_collected = HashMap::new();
		let mut counter = 0;
		let mut is_successs = false;
		let mut is_failure = false;
		loop {
			tokio::select! {
				event_opt = self.event_receiver.recv() => {
					if let Some((event_type, event_data)) = event_opt {
						if event_data.payload.is_empty() {
							continue;
						}
						if event_type == EventType::Success {
							is_successs = true;
							break
						}
						if event_type == EventType::Failure {
							is_failure = true;
							break
						}
						self.counters.increment_total();
						// check if the event type is already in the map
						if events_collected.contains_key(&event_type) {
							let event_vec: &mut Vec<EventState> = events_collected.get_mut(&event_type).unwrap();
							event_vec.push(event_data);
						} else {
							events_collected.insert(event_type, vec![event_data]);
						}
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
			let batches_error =
				ctx.send_message(event_streamer_messagebus, events_batch.clone()).await;
			if batches_error.is_err() {
				log::error!("Failed to send events batch: {:?}", batches_error);
				//re-trying
				let retry_error = ctx.send_message(event_streamer_messagebus, events_batch).await;
				if retry_error.is_err() {
					return Err(ActorExitStatus::Failure(
						anyhow::anyhow!("Failed to send events batch: {:?}", retry_error).into(),
					));
				}
			}
		}
		if is_successs {
			return Err(ActorExitStatus::Success);
		}
		if is_failure {
			return Err(ActorExitStatus::Failure(anyhow::anyhow!("Qflow failed").into()));
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
		self.workflow_handle.take().unwrap().abort();
		if self.collection_handler.is_none() {
			return Ok(());
		}
		self.collection_handler.take().unwrap().kill().await;
		Ok(())
	}
}
