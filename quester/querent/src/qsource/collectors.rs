use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use querent_synapse::{
	callbacks::{EventState, EventType},
	comm::IngestedTokens,
	config::Config,
	querent::{Querent, QuerentError, Workflow, WorkflowBuilder},
};
use std::time::Duration;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{error, info};

use crate::{EventLock, EventStreamer, NewEventLock, Source, SourceContext};

pub struct Collection {
	pub id: String,
	pub workflow: Workflow,
	pub event_lock: EventLock,
	token_sender: Option<crossbeam_channel::Sender<IngestedTokens>>,
	event_sender: mpsc::Sender<(EventType, EventState)>,
	workflow_handle: Option<JoinHandle<Result<(), QuerentError>>>,
}

const _CODE_CONFIG_COLLECTION: &str = r#"
import asyncio
import json

async def print_querent(config, text: str):
    print(text + ": Collection Bot 🤖")
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
}

#[async_trait]
impl Source for Collection {
	async fn initialize(
		&mut self,
		_event_streamer_messagebus: &MessageBus<EventStreamer>,
		ctx: &SourceContext,
	) -> Result<(), ActorExitStatus> {
		if self.workflow_handle.is_some() {
			if self.workflow_handle.as_ref().unwrap().is_finished() {
				error!("Collection for current pipeline is already finished");
				return Err(ActorExitStatus::Success);
			}
			return Ok(());
		}

		info!("Starting data collection: 📚");
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

		// Store the JoinHandle with the result in the QSource struct
		self.workflow_handle = Some(tokio::spawn(async move {
			let result = querent.start_workflows().await;
			match result {
				Ok(()) => {
					// Handle the success
					log::info!(
						"Successfully started the collection workflow with id: {}",
						workflow_id
					);
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
		ctx.send_message(_event_streamer_messagebus, NewEventLock(event_lock)).await?;
		Ok(())
	}

	async fn emit_events(
		&mut self,
		_event_streamer_messagebus: &MessageBus<EventStreamer>,
		_ctx: &SourceContext,
	) -> Result<Duration, ActorExitStatus> {
		if self.workflow_handle.is_none() {
			return Err(ActorExitStatus::Success);
		}
		Ok(Duration::default())
	}
	fn name(&self) -> String {
		format!("Collection: {}", self.id)
	}

	fn observable_state(&self) -> serde_json::Value {
		serde_json::json!({
			"workflow_id": self.workflow.id,
		})
	}

	async fn finalize(
		&mut self,
		_exit_status: &ActorExitStatus,
		_ctx: &SourceContext,
	) -> anyhow::Result<()> {
		self.workflow_handle.take().unwrap().abort();
		Ok(())
	}
}
