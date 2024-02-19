use actors::{Actor, ActorContext, ActorExitStatus, QueueCapacity};
use async_trait::async_trait;
use common::{EventsCounter, RuntimeType};
use querent_synapse::{
	callbacks::{EventState, EventType},
	comm::IngestedTokens,
	config::Config,
	querent::{Querent, QuerentError, Workflow, WorkflowBuilder},
};
use std::sync::Arc;
use tokio::{runtime::Handle, sync::mpsc, task::JoinHandle};
use tracing::{error, info};

use crate::EventLock;

pub struct Collection {
	pub id: String,
	pub workflow: Workflow,
	pub event_lock: EventLock,
	pub counters: Arc<EventsCounter>,
	token_sender: Option<crossbeam_channel::Sender<IngestedTokens>>,
	event_sender: mpsc::Sender<(EventType, EventState)>,
	workflow_handle: Option<JoinHandle<Result<(), QuerentError>>>,
}

const _CODE_CONFIG_COLLECTION: &str = r#"
import asyncio
import json

async def print_querent(config, text: str):
    print("Collection Bot 🤖" + text)
    querent_started = False

    try:
        import querent
        print("✨ Querent for ingestion imported successfully ✨")
        querent_started = True
        await querent.workflow.start_ingestion(config)
        return
    except Exception as e:
        querent_started = False
        print("❌ Failed to import querent for ingestion: " + str(e))

"#;

impl Collection {
	pub fn new(
		id: String,
		workflow: Workflow,
		event_sender: mpsc::Sender<(EventType, EventState)>,
		token_sender: Option<crossbeam_channel::Sender<IngestedTokens>>,
	) -> Self {
		let workflow = WorkflowBuilder::new(workflow.id.as_str())
			.name(workflow.name.as_str())
			.import(Some(workflow.import))
			.attr(Some(workflow.attr))
			.code(Some(_CODE_CONFIG_COLLECTION.to_string()))
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
			workflow_handle: None,
		}
	}

	pub fn get_token_sender(&self) -> Option<crossbeam_channel::Sender<IngestedTokens>> {
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
impl Actor for Collection {
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
		// Store the JoinHandle with the result in the Qflow struct
		let event_sender = self.event_sender.clone();
		info!("Starting the workflow with id: {}", workflow_id);
		self.workflow_handle = Some(tokio::spawn(async move {
			let result: Result<(), QuerentError> = querent.start_workflows().await;
			match result {
				Ok(()) => {
					// Handle the success
					info!("Successfully started the workflow with id: {}", workflow_id);
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
					error!(
						"Failed to run the workflow with id: {} and error: {:?}",
						workflow_id, err
					);
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
		Ok(())
	}
	fn observable_state(&self) -> Self::ObservableState {
		()
	}

	fn queue_capacity(&self) -> QueueCapacity {
		QueueCapacity::Bounded(10)
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
