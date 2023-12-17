use actors::ActorExitStatus;
use async_trait::async_trait;
use log;
use querent_rs::callbacks::interface::EventHandler;
use querent_rs::callbacks::{EventState, EventType};
use querent_rs::config::config::WorkflowConfig;
use querent_rs::config::Config;
use querent_rs::querent::{Querent, QuerentError, Workflow, WorkflowBuilder};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self};

use crate::{EventLock, Source, SourceContext, BATCH_NUM_EVENTS_LIMIT, EMIT_BATCHES_TIMEOUT};

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

    pub fn increment_processed(&self, count: u64) {
        self.processed.fetch_add(count, Ordering::SeqCst);
    }
}

pub struct Qflow {
    pub id: String,
    pub workflow: Workflow,
    pub publish_lock: EventLock,
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
            publish_lock: EventLock::default(),
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
    async fn initialize(&mut self, _ctx: &SourceContext) -> Result<(), ActorExitStatus> {
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
                            },
                        ))
                        .await
                        .unwrap();
                    Ok(())
                }
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
                            },
                        ))
                        .await
                        .unwrap();
                    Err(err)
                }
            }
        }));

        Ok(())
    }

    async fn emit_events(&mut self, ctx: &SourceContext) -> Result<Duration, ActorExitStatus> {
        let deadline = time::sleep(EMIT_BATCHES_TIMEOUT);
        tokio::pin!(deadline);
        let mut events_collected = HashMap::new();
        loop {
            tokio::select! {
                event_opt = self.event_receiver.recv() => {
                    if let Some((event_type, event_data)) = event_opt {
                        if event_type == EventType::Success {
                            return Err(ActorExitStatus::Success)
                        }
                        if event_type == EventType::Failure {
                            return Err(ActorExitStatus::Failure(anyhow::anyhow!(event_data.payload).into()))
                        }
                        self.counters.increment_total();
                        events_collected.insert(event_type, event_data);
                    }
                    if events_collected.len() >= BATCH_NUM_EVENTS_LIMIT {
                        self.counters.increment_processed(events_collected.len() as u64);
                        break;
                    }
                    ctx.record_progress();
                }
                _ = &mut deadline => {
                    break;
                }
            }
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
