use actors::ActorExitStatus;
use async_trait::async_trait;
use querent_rs::callbacks::interface::EventHandler;
use querent_rs::callbacks::{EventState, EventType};
use querent_rs::config::config::WorkflowConfig;
use querent_rs::config::Config;
use querent_rs::querent::{Querent, QuerentError, Workflow, WorkflowBuilder};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::{EventLock, Source, SourceContext};

#[derive(Debug, Serialize)]
pub struct EventsCounter {
    pub qflow_id: String,
    pub total: AtomicU64,
    pub processed: AtomicU64,
}

impl EventsCounter {
    pub fn new(qflow_id: String) -> Self {
        Self {
            qflow_id,
            total: AtomicU64::new(0),
            processed: AtomicU64::new(0),
        }
    }

    pub fn increment_total(&self) {
        self.total.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_processed(&self) {
        self.processed.fetch_add(1, Ordering::SeqCst);
    }
}

pub struct Qflow {
    pub id: String,
    pub workflow: Workflow,
    pub publish_lock: EventLock,
    pub counters: Arc<EventsCounter>,
    event_receiver: mpsc::Receiver<(EventType, EventState)>,
    pub workflow_handle: Option<JoinHandle<Result<(), QuerentError>>>,
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
            .build();
        Self {
            id: id.clone(),
            workflow,
            publish_lock: EventLock::default(),
            counters: Arc::new(EventsCounter::new(id.clone())),
            event_receiver,
            workflow_handle: None,
        }
    }

    fn process_event(&self, event_type: EventType, event_data: EventState) {
        // Increment the total events counter
        self.counters.increment_total();
        eprintln!("{}: Processing event - {:?}", self.id, event_type);
        eprintln!("{}: Processing event - {:?}", self.id, event_data);
        // Increment the processed events counter
        self.counters.increment_processed();
    }

    pub fn get_config(&self) -> Option<Config> {
        self.workflow.config.clone()
    }
}

#[async_trait]
impl Source for Qflow {
    async fn initialize(&mut self, _ctx: &SourceContext) -> Result<(), ActorExitStatus> {
        let querent = Querent::new().map_err(|e| {
            ActorExitStatus::Failure(
                anyhow::anyhow!("Failed to initialize querent: {:?}", e).into(),
            )
        })?;
        querent.add_workflow(self.workflow.clone()).map_err(|e| {
            ActorExitStatus::Failure(anyhow::anyhow!("Failed to add workflow: {:?}", e).into())
        })?;

        // Store the JoinHandle with the result in the Qflow struct
        self.workflow_handle = Some(tokio::spawn(async move {
            let result = querent.start_workflows().await;
            match result {
                Ok(()) => {
                    // Handle the success
                    eprintln!("Successfully started workflows");
                    Ok(())
                }
                Err(err) => {
                    // Handle the error, e.g., log it
                    eprintln!("Error starting workflows: {:?}", err);
                    Err(err)
                }
            }
        }));

        Ok(())
    }

    async fn emit_batches(&mut self, _ctx: &SourceContext) -> Result<Duration, ActorExitStatus> {
        loop {
            tokio::select! {
                event_opt = self.event_receiver.recv() => {
                    if let Some((event_type, event_data)) = event_opt {
                        self.process_event(event_type, event_data);
                    }
                }
                _ = sleep(Duration::from_secs(1)) => {
                    // Placeholder for additional logic in the loop
                }
            }
        }
    }

    fn name(&self) -> String {
        self.id.clone()
    }

    fn observable_state(&self) -> serde_json::Value {
        serde_json::to_value(&self.counters).unwrap()
    }
}
