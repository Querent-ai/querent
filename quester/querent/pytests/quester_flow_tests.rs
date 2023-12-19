use std::{collections::HashMap, sync::atomic::Ordering};

use actors::Quester;
use querent::{EventStreamer, Qflow, SourceActor};
use querent_synapse::{
	config::{config::WorkflowConfig, Config},
	cross::{CLRepr, StringType},
	querent::workflow::Workflow,
};

#[pyo3_asyncio::tokio::main]
async fn main() -> pyo3::PyResult<()> {
	pyo3_asyncio::testing::main().await
}

const CODE_CONFIG_EVENT_HANDLER: &str = r#"
import asyncio

async def print_querent(config, text: str):
    """Prints the provided text and sends supported event_type and event_data"""
    print(text)
    if config['workflow'] is not None:
        event_type = "ContextualTriples"  # Replace with the desired event type
        event_data = {
            "event_type": event_type,
            "timestamp": 123.45,  # Replace with the actual timestamp
            "payload": "🚀🚀",  # Replace with the actual payload data
			"file": "file_name"  # Replace with the actual file name
        }
        config['workflow']['event_handler'].handle_event(event_type, event_data)
"#;

#[pyo3_asyncio::tokio::test]
async fn qflow_basic_message_bus() -> pyo3::PyResult<()> {
	let quester = Quester::with_accelerated_time();
	let (event_streamer_messagebus, indexer_inbox) = quester.create_test_messagebus();
	let config = Config {
		version: 1.0,
		querent_id: "event_handler".to_string(),
		querent_name: "Test Querent event_handler".to_string(),
		workflow: WorkflowConfig {
			name: "test_workflow".to_string(),
			id: "workflow_id".to_string(),
			config: HashMap::new(),
			channel: None,
			inner_channel: None,
			inner_event_handler: None,
			event_handler: None,
		},
		collectors: vec![],
		engines: vec![],
		resource: None,
	};

	// Create a sample Workflow
	let workflow = Workflow {
		name: "test_workflow".to_string(),
		id: "workflow_id".to_string(),
		import: "".to_string(),
		attr: "print_querent".to_string(),
		code: Some(CODE_CONFIG_EVENT_HANDLER.to_string()),
		arguments: vec![CLRepr::String("Querent".to_string(), StringType::Normal)],
		config: Some(config),
	};

	// Create a sample Qflow
	let qflow_actor = Qflow::new("qflow_id".to_string(), workflow);

	// Initialize the Qflow
	let qflow_source_actor =
		SourceActor { source: Box::new(qflow_actor), event_streamer_messagebus };

	let (_, qflow_source_handle) = quester.spawn_builder().spawn(qflow_source_actor);
	let (actor_termination, _) = qflow_source_handle.join().await;
	assert!(actor_termination.is_success());

	let drained_messages = indexer_inbox.drain_for_test();

	// Verify that the event handler sent the expected event
	assert_eq!(drained_messages.len(), 2);

	Ok(())
}

#[pyo3_asyncio::tokio::test]
async fn qflow_with_streamer_message_bus() -> pyo3::PyResult<()> {
	let quester = Quester::with_accelerated_time();
	let config = Config {
		version: 1.0,
		querent_id: "event_handler".to_string(),
		querent_name: "Test Querent event_handler".to_string(),
		workflow: WorkflowConfig {
			name: "test_workflow".to_string(),
			id: "workflow_id".to_string(),
			config: HashMap::new(),
			channel: None,
			inner_channel: None,
			inner_event_handler: None,
			event_handler: None,
		},
		collectors: vec![],
		engines: vec![],
		resource: None,
	};

	// Create a sample Workflow
	let workflow = Workflow {
		name: "test_workflow".to_string(),
		id: "workflow_id".to_string(),
		import: "".to_string(),
		attr: "print_querent".to_string(),
		code: Some(CODE_CONFIG_EVENT_HANDLER.to_string()),
		arguments: vec![CLRepr::String("Querent".to_string(), StringType::Normal)],
		config: Some(config),
	};

	// Create a sample Qflow
	let qflow_actor = Qflow::new("qflow_id".to_string(), workflow);

	// Create storage mapper message bus
	let (storage_mapper_messagebus, _) = quester.create_test_messagebus();
	// Create a EventStreamer
	let event_streamer_actor =
		EventStreamer::new("qflow_id".to_string(), storage_mapper_messagebus, 0);

	let (event_streamer_messagebus, event_handle) =
		quester.spawn_builder().spawn(event_streamer_actor);

	// Initialize the Qflow
	let qflow_source_actor =
		SourceActor { source: Box::new(qflow_actor), event_streamer_messagebus };

	let (_, qflow_source_handle) = quester.spawn_builder().spawn(qflow_source_actor);
	let (actor_termination, _) = qflow_source_handle.join().await;
	assert!(actor_termination.is_success());

	let observed_state = event_handle.process_pending_and_observe().await.state;

	// Verify that the event handler sent the expected event
	assert_eq!(observed_state.events_received.load(Ordering::Relaxed), 1);

	Ok(())
}
