use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{EventsCounter, RuntimeType};
use querent_synapse::{
	callbacks::{EventState, EventType},
	comm::IngestedTokens,
	config::Config,
	querent::{Querent, QuerentError, Workflow, WorkflowBuilder},
};
use std::{sync::Arc, time::Duration};
use tokio::{runtime::Handle, sync::mpsc, task::JoinHandle, time};
use tracing::{error, info};

use crate::EventLock;

pub struct QueryEngine {
	pub id: String,
	pub workflow: Workflow,
	pub event_lock: EventLock,
	pub counters: Arc<EventsCounter>,
	token_sender: crossbeam_channel::Sender<IngestedTokens>,
	event_sender: mpsc::Sender<(EventType, EventState)>,
	event_receiver: mpsc::Receiver<(EventType, EventState)>,
	workflow_handle: Option<JoinHandle<Result<(), QuerentError>>>,
}

const _CODE_CONFIG_QUERY: &str = r#"
import asyncio
import json

async def print_querent(config, text: str):
    print(text + ": Query Bot 🤖")
    querent_started = False

    try:
        import querent
        print("✨ Querent for query engine import successfully ✨")
        querent_started = True
        querent.workflow.start_graph_query_engine(config)
        return
    except Exception as e:
        querent_started = False
        print("❌ Failed to import querent for query engine: " + str(e))

"#;

pub const QUERY_BATCHES_TIMEOUT: Duration =
	Duration::from_millis(if cfg!(test) { 100 } else { 5_000 });

pub const BATCH_NUM_LIMIT: usize = 1;

impl QueryEngine {
	pub fn new(
		id: String,
		workflow: Workflow,
		event_sender: mpsc::Sender<(EventType, EventState)>,
		event_receiver: mpsc::Receiver<(EventType, EventState)>,
		token_sender: crossbeam_channel::Sender<IngestedTokens>,
	) -> Self {
		let workflow = WorkflowBuilder::new(workflow.id.as_str())
			.name(workflow.name.as_str())
			.import(Some(workflow.import))
			.attr(Some(workflow.attr))
			.code(Some(_CODE_CONFIG_QUERY.to_string()))
			.config(workflow.config.unwrap_or(Config::default()))
			.arguments(workflow.arguments)
			.build();
		Self {
			id: id.clone(),
			workflow,
			event_lock: EventLock::default(),
			counters: Arc::new(EventsCounter::default()),
			token_sender,
			event_sender,
			event_receiver,
			workflow_handle: None,
		}
	}

	pub fn get_token_sender(&self) -> crossbeam_channel::Sender<IngestedTokens> {
		self.token_sender.clone()
	}

	pub fn get_id(&self) -> String {
		self.id.clone()
	}

	pub fn get_workflow(&self) -> Workflow {
		self.workflow.clone()
	}

	pub fn get_event_lock(&self) -> EventLock {
		self.event_lock.clone()
	}

	pub fn get_counters(&self) -> Arc<EventsCounter> {
		self.counters.clone()
	}
}

#[async_trait]
impl Actor for QueryEngine {
	type ObservableState = ();

	async fn initialize(&mut self, _ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		if self.workflow_handle.is_some() {
			if self.workflow_handle.as_ref().unwrap().is_finished() {
				error!("Collection is already finished");
				return Err(ActorExitStatus::Success);
			}
			return Ok(());
		}
		let querent = Querent::new().map_err(|e| {
			ActorExitStatus::Failure(
				anyhow::anyhow!("Failed to initialize collection task: {:?}", e).into(),
			)
		})?;
		querent.add_workflow(self.workflow.clone()).map_err(|e| {
			ActorExitStatus::Failure(
				anyhow::anyhow!("Failed to add collection workflow: {:?}", e).into(),
			)
		})?;

		let workflow_id = self.workflow.id.clone();
		// Store the JoinHandle with the result in the QSource struct
		let event_sender = self.event_sender.clone();
		info!("Starting the workflow with id: {}", workflow_id);
		self.workflow_handle = Some(tokio::spawn(async move {
			let result: Result<(), QuerentError> = querent.start_workflows().await;
			match result {
				Ok(()) => {
					// Handle the success
					info!("Successfully started the query engine with id: {}", workflow_id);
					log::info!("Successfully started the query engine with id: {}", workflow_id);
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
					error!(
						"Failed to run the query engine with id: {} and error: {:?}",
						workflow_id, err
					);
					log::error!(
						"Failed to run the query engine with id: {} and error: {:?}",
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
		Ok(())
	}
	fn observable_state(&self) -> Self::ObservableState {
		()
	}

	fn queue_capacity(&self) -> QueueCapacity {
		QueueCapacity::Bounded(1000)
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
		_exit_status: &ActorExitStatus,
		_ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		self.workflow_handle.take().unwrap().abort();
		Ok(())
	}
}

#[async_trait]
impl Handler<IngestedTokens> for QueryEngine {
	type Reply = Vec<(EventType, EventState)>;

	async fn handle(
		&mut self,
		ingested_tokens: IngestedTokens,
		ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		self.token_sender.send(ingested_tokens).unwrap();
		// wait for tokens to be processed and receive events
		let deadline = time::sleep(QUERY_BATCHES_TIMEOUT);
		tokio::pin!(deadline);
		let mut events_collected = Vec::new();
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
						if event_type == EventType::QueryResult {
							self.counters.increment_total();
							events_collected.push((event_type, event_data));
							counter += 1;
						}
					}
					if counter >= BATCH_NUM_LIMIT {
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
		if is_successs {
			return Err(ActorExitStatus::Success);
		}
		if is_failure {
			return Err(ActorExitStatus::Failure(anyhow::anyhow!("QSource failed").into()));
		}
		Ok(events_collected)
	}
}
