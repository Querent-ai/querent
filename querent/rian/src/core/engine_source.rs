use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use common::{EventState, EventType, EventsBatch, EventsCounter, TerimateSignal};
use engines::{Engine, EngineError, EngineErrorKind};
use futures::StreamExt;
use proto::semantics::IngestedTokens;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
	sync::mpsc,
	task::JoinHandle,
	time::{self},
};
use tracing::{error, info};

use crate::{
	EventLock, EventStreamer, NewEventLock, Source, SourceContext, BATCH_NUM_EVENTS_LIMIT,
	EMIT_BATCHES_TIMEOUT,
};

pub struct EngineRunner {
	pub id: String,
	pub engine: Arc<dyn Engine>,
	pub event_lock: EventLock,
	pub counters: Arc<EventsCounter>,
	event_receiver: Option<mpsc::Receiver<(EventType, EventState)>>,
	workflow_handle: Option<JoinHandle<Result<(), EngineError>>>,
	// terimatesignal to kill actors in the pipeline.
	pub terminate_sig: TerimateSignal,
}

impl EngineRunner {
	pub fn new(
		id: String,
		engine: Arc<dyn Engine>,
		token_receiver: mpsc::Receiver<IngestedTokens>,
		terminate_sig: TerimateSignal,
	) -> Self {
		let (event_sender, event_receiver) = mpsc::channel(1000);
		let event_runner = engine.clone();
		info!("Starting the engine 🚀");
		let workflow_handle = Some(tokio::spawn(async move {
			let mut engine_op = event_runner
				.process_ingested_tokens(token_receiver)
				.await
				.map_err(|e| {
					ActorExitStatus::Failure(
						anyhow::anyhow!("Failed to process ingested tokens: {:?}", e).into(),
					)
				})
				.expect("Expect engine to run");

			while let Some(data) = engine_op.next().await {
				match data {
					Ok(event) => {
						if let Err(e) = event_sender.send((event.clone().event_type, event)).await {
							return Err(EngineError::new(
								EngineErrorKind::EventStream,
								Arc::new(anyhow::anyhow!("Failed to send event: {:?}", e)),
							));
						}
					},
					Err(e) => match e.kind() {
						EngineErrorKind::EventStream => {
							error!("Failed to process ingested tokens: {:?}", e);
							let fail_event = EventState {
								event_type: EventType::Failure,
								timestamp: chrono::Utc::now().timestamp_millis() as f64,
								payload: format!("{:?}", e),
								file: "".to_string(),
								doc_source: "".to_string(),
								image_id: None,
							};

							if let Err(e) =
								event_sender.send((fail_event.clone().event_type, fail_event)).await
							{
								error!("Failed to send event: {:?}", e);
								return Err(EngineError::new(
									EngineErrorKind::EventStream,
									Arc::new(anyhow::anyhow!("Failed to send event: {:?}", e)),
								));
							}
							break;
						},
						_ => {
							error!("Failed to process ingested tokens: {:?}", e);
							return Err(EngineError::new(
								EngineErrorKind::EventStream,
								Arc::new(anyhow::anyhow!("Failed to send event: {:?}", e)),
							));
						},
					},
				}
			}

			let success_event = EventState {
				event_type: EventType::Success,
				timestamp: chrono::Utc::now().timestamp_millis() as f64,
				payload: "".to_string(),
				file: "".to_string(),
				doc_source: "".to_string(),
				image_id: None,
			};

			if let Err(e) =
				event_sender.send((success_event.clone().event_type, success_event)).await
			{
				error!("Failed to send event: {:?}", e);
				return Err(EngineError::new(
					EngineErrorKind::EventStream,
					Arc::new(anyhow::anyhow!("Failed to send event: {:?}", e)),
				));
			}
			Ok(())
		}));
		Self {
			id: id.clone(),
			engine,
			event_lock: EventLock::default(),
			counters: Arc::new(EventsCounter::new(id.clone())),
			event_receiver: Some(event_receiver),
			workflow_handle,
			terminate_sig,
		}
	}

	pub fn get_engine(&self) -> Arc<dyn Engine> {
		self.engine.clone()
	}
}

#[async_trait]
impl Source for EngineRunner {
	async fn initialize(
		&mut self,
		event_streamer_messagebus: &MessageBus<EventStreamer>,
		ctx: &SourceContext,
	) -> Result<(), ActorExitStatus> {
		if self.workflow_handle.is_some() {
			if self.workflow_handle.as_ref().unwrap().is_finished() {
				error!("EngineRunner is already finished");
				return Err(ActorExitStatus::Success);
			}
			return Ok(());
		}

		info!("Started the engine 🚀🚀 with id: {}", self.id);
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
		let event_receiver = self.event_receiver.as_mut().unwrap();

		loop {
			tokio::select! {
				event_opt = event_receiver.recv() => {
					if let Some((event_type, event_data)) = event_opt {
						if event_data.payload.is_empty() {
							continue;
						}
						if event_type == EventType::Success {
							is_successs = true;
							// clear the receiver
							self.event_receiver.take();
							break
						}
						if event_type == EventType::Failure {
							error!("EngineRunner failed");
							self.event_receiver.take();
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
					self.counters.increment_processed(counter as u64);
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
			let batches_error = ctx.send_message(event_streamer_messagebus, events_batch).await;
			if batches_error.is_err() {
				error!("Failed to send events batch: {:?}", batches_error);
			}
		}

		if is_successs {
			// sleep for 10 seconds to allow the engine send remaining events to database
			time::sleep(Duration::from_secs(10)).await;
			return Err(ActorExitStatus::Success);
		}
		if is_failure {
			return Err(ActorExitStatus::Failure(anyhow::anyhow!("EngineRunner failed").into()));
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
		exit_status: &ActorExitStatus,
		_ctx: &SourceContext,
	) -> anyhow::Result<()> {
		log::info!("Engine Runner with id: {} is finalizing", self.id);
		match self.workflow_handle.take() {
			Some(handle) => {
				info!("EngineRunner is finalizing with status: {:?}", exit_status);
				handle.abort();
			},
			None => {
				info!("EngineRunner is already finished");
			},
		};
		Ok(())
	}
}
