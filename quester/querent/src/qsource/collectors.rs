use actors::{ActorExitStatus, MessageBus};
use async_trait::async_trait;
use common::{EventsBatch, EventsCounter};
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
use tracing::error;


pub struct Collection {
	pub id: String,
	pub workflow: Workflow,
	pub event_lock: EventLock,
	pub counters: Arc<EventsCounter>,
	token_sender: Option<crossbeam_channel::Sender<IngestedTokens>>,
	workflow_handle: Option<JoinHandle<Result<(), QuerentError>>>,
}


