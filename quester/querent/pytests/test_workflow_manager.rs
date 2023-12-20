use std::collections::HashMap;

use actors::Quester;
use pyo3::exceptions::PyTypeError;
use querent::{Qflow, SourceActor};
use querent_synapse::{
	callbacks::{interface::EventHandler, EventType},
	comm::ChannelHandler,
	config::{config::WorkflowConfig, Config},
	cross::{CLRepr, StringType},
	querent::workflow::{Workflow, WorkflowManager},
};

#[pyo3_asyncio::tokio::main]
async fn main() -> pyo3::PyResult<()> {
	pyo3_asyncio::testing::main().await
}

#[pyo3_asyncio::tokio::test]
async fn test_basic() -> pyo3::PyResult<()> {
	Ok(())
}

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_basic_tests() -> pyo3::PyResult<()> {
	let workflow_manager_res = WorkflowManager::new();
	if let Err(e) = workflow_manager_res {
		panic!("Error creating workflow manager: {}", e);
	}
	let workflow_manager = workflow_manager_res.unwrap();
	//let config = Config::default();
	let mut args: Vec<CLRepr> = Vec::new();
	args.push(CLRepr::Int(1));
	let test_flow: Workflow = Workflow {
		name: "test_flow_basic".to_string(),
		id: "id1".to_string(),
		import: "asyncio".to_string(),
		attr: "sleep".to_string(),
		arguments: args,
		code: None,
		config: None,
	};
	assert!(workflow_manager.add_workflow(test_flow).is_ok());
	match workflow_manager.start_workflows().await {
		Ok(_) => assert!(true),
		Err(e) => panic!("Error starting workflows: {}", e),
	}
	Ok(())
}

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_multiple_workflows() -> pyo3::PyResult<()> {
	let workflow_manager_res = WorkflowManager::new();
	if let Err(e) = workflow_manager_res {
		panic!("Error creating workflow manager: {}", e);
	}
	let workflow_manager = workflow_manager_res.unwrap();

	// Add multiple workflows
	let mut args1: Vec<CLRepr> = Vec::new();
	args1.push(CLRepr::Int(1));
	let test_flow1 = Workflow {
		name: "test_flow_1".to_string(),
		id: "id3".to_string(),
		import: "asyncio".to_string(),
		attr: "sleep".to_string(),
		arguments: args1,
		code: None,
		config: None,
	};
	assert!(workflow_manager.add_workflow(test_flow1).is_ok());

	let mut args2: Vec<CLRepr> = Vec::new();
	args2.push(CLRepr::Int(2));
	let test_flow2 = Workflow {
		name: "test_flow_2".to_string(),
		id: "id4".to_string(),
		import: "asyncio".to_string(),
		attr: "sleep".to_string(),
		arguments: args2,
		code: None,
		config: None,
	};
	assert!(workflow_manager.add_workflow(test_flow2).is_ok());

	// Start workflows
	match workflow_manager.start_workflows().await {
		Ok(_) => assert!(true),
		Err(e) => panic!("Error starting workflows: {}", e),
	}
	Ok(())
}

const _CODE: &str = r#"
import asyncio

async def print_querent(text: str) -> str:
    """Attempt to parse and execute input as Python"""
    await asyncio.sleep(1)  # Simulate asynchronous behavior
    print(text)
"#;

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_tests() -> pyo3::PyResult<()> {
	let workflow_manager_res = WorkflowManager::new();
	if let Err(e) = workflow_manager_res {
		panic!("Error creating workflow manager: {}", e);
	}
	let workflow_manager = workflow_manager_res.unwrap();
	//let config = Config::default();
	let mut args: Vec<CLRepr> = Vec::new();
	args.push(CLRepr::String("Querent".to_string(), StringType::Normal));
	let test_flow: Workflow = Workflow {
		name: "test_flow_python".to_string(),
		id: "id2".to_string(),
		import: "".to_string(),
		attr: "print_querent".to_string(),
		arguments: args,
		code: Some(_CODE.to_string()),
		config: None,
	};
	assert!(workflow_manager.add_workflow(test_flow).is_ok());
	match workflow_manager.start_workflows().await {
		Ok(_) => assert!(true),
		Err(e) => panic!("Error starting workflows: {}", e),
	}
	Ok(())
}

const CODE_WITH_RESULT: &str = r#"
import asyncio

async def add_numbers(a, b):
    await asyncio.sleep(1)  # Simulate asynchronous behavior
    print(a + b)
    return a + b
"#;

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_test_with_result() -> pyo3::PyResult<()> {
	let workflow_manager_res = WorkflowManager::new();
	if let Err(e) = workflow_manager_res {
		panic!("Error creating workflow manager: {}", e);
	}
	let workflow_manager = workflow_manager_res.unwrap();

	let mut args: Vec<CLRepr> = Vec::new();
	args.push(CLRepr::Int(3));
	args.push(CLRepr::Int(4));
	let test_flow = Workflow {
		name: "test_flow_with_result".to_string(),
		id: "id5".to_string(),
		import: "".to_string(),
		attr: "add_numbers".to_string(),
		arguments: args,
		code: Some(CODE_WITH_RESULT.to_string()),
		config: None,
	};
	assert!(workflow_manager.add_workflow(test_flow).is_ok());

	// Start workflows
	match workflow_manager.start_workflows().await {
		Ok(_) => {
			assert!(true);
		},
		Err(e) => panic!("Error starting workflows: {}", e),
	}
	Ok(())
}

const CODE_CONFIG: &str = r#"
import asyncio

async def print_querent(config, text):
    """Prints the provided text and config"""
    print(text)
    print(config['querent_name'])
"#;

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_tests_with_config() -> pyo3::PyResult<()> {
	// Create a sample Config object
	let config = Config {
		version: 1.0,
		querent_id: "test_querent".to_string(),
		querent_name: "Test Querent Callback".to_string(),
		workflow: WorkflowConfig {
			name: "test_workflow".to_string(),
			id: "workflow_id".to_string(),
			config: HashMap::new(),
			channel: None,
			inner_channel: Some(ChannelHandler::new()),
			inner_event_handler: Some(EventHandler::new(None)),
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
		code: Some(CODE_CONFIG.to_string()),
		arguments: vec![CLRepr::String("Querent".to_string(), StringType::Normal)],
		config: Some(config),
	};

	// Create a WorkflowManager and add the Workflow
	let workflow_manager = WorkflowManager::new().expect("Failed to create WorkflowManager");
	assert!(workflow_manager.add_workflow(workflow).is_ok());

	// Start the workflows
	match workflow_manager.start_workflows().await {
		Ok(_) => assert!(true),
		Err(e) => panic!("Error starting workflows: {}", e),
	}

	Ok(())
}

const CODE_CONFIG_2: &str = r#"
import asyncio

async def print_querent(config, text):
    """Prints the provided text and config"""
    print(text)
    print(config['workflow'])
"#;

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_tests_with_config2() -> pyo3::PyResult<()> {
	// Create a sample Config object
	let config = Config {
		version: 1.0,
		querent_id: "event_handler".to_string(),
		querent_name: "Test Querent event_handler".to_string(),
		workflow: WorkflowConfig {
			name: "test_workflow".to_string(),
			id: "workflow_id".to_string(),
			config: HashMap::new(),
			channel: None,
			inner_channel: Some(ChannelHandler::new()),
			inner_event_handler: Some(EventHandler::new(None)),
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
		code: Some(CODE_CONFIG_2.to_string()),
		arguments: vec![CLRepr::String("Querent".to_string(), StringType::Normal)],
		config: Some(config),
	};

	// Create a WorkflowManager and add the Workflow
	let workflow_manager = WorkflowManager::new().expect("Failed to create WorkflowManager");
	assert!(workflow_manager.add_workflow(workflow).is_ok());

	// Start the workflows
	match workflow_manager.start_workflows().await {
		Ok(_) => assert!(true),
		Err(e) => panic!("Error starting workflows: {}", e),
	}

	Ok(())
}

const CODE_CONFIG_CHANNEL: &str = r#"
import asyncio

async def print_querent(config, text):
    """Prints the provided text and config"""
    print(text)
    print(config['workflow']['channel'].receive_in_python())
"#;

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_tests_with_config_channel() -> pyo3::PyResult<()> {
	// Create a sample Config object
	let config = Config {
		version: 1.0,
		querent_id: "event_handler".to_string(),
		querent_name: "Test Querent event_handler".to_string(),
		workflow: WorkflowConfig {
			name: "test_workflow".to_string(),
			id: "workflow_id".to_string(),
			config: HashMap::new(),
			channel: None,
			inner_channel: Some(ChannelHandler::new()),
			inner_event_handler: Some(EventHandler::new(None)),
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
		code: Some(CODE_CONFIG_CHANNEL.to_string()),
		arguments: vec![CLRepr::String("Querent".to_string(), StringType::Normal)],
		config: Some(config),
	};

	// Create a WorkflowManager and add the Workflow
	let workflow_manager = WorkflowManager::new().expect("Failed to create WorkflowManager");
	assert!(workflow_manager.add_workflow(workflow).is_ok());

	// Start the workflows
	match workflow_manager.start_workflows().await {
		Ok(_) => assert!(true),
		Err(e) => panic!("Error starting workflows: {}", e),
	}

	Ok(())
}

const CODE_CONFIG_EVENT_HANDLER: &str = r#"
import asyncio

async def print_querent(config, text: str):
    """Prints the provided text and sends supported event_type and event_data"""
    print(text)
    if config['workflow'] is not None:
        event_type = "Graph"  # Replace with the desired event type
        event_data = {
            "event_type": event_type,
            "timestamp": 123.45,  # Replace with the actual timestamp
            "payload": "🚀🚀",  # Replace with the actual payload data
			"file": "test_file"
        }
        config['workflow']['event_handler'].handle_event(event_type, event_data)
"#;

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_tests_with_config_events() -> pyo3::PyResult<()> {
	// Create a sample Config object
	let config = Config {
		version: 1.0,
		querent_id: "event_handler".to_string(),
		querent_name: "Test Querent event_handler".to_string(),
		workflow: WorkflowConfig {
			name: "test_workflow".to_string(),
			id: "workflow_id".to_string(),
			config: HashMap::new(),
			channel: None,
			inner_channel: Some(ChannelHandler::new()),
			inner_event_handler: Some(EventHandler::new(None)),
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

	// Create a WorkflowManager and add the Workflow
	let workflow_manager = WorkflowManager::new().expect("Failed to create WorkflowManager");
	assert!(workflow_manager.add_workflow(workflow).is_ok());

	// Start the workflows
	match workflow_manager.start_workflows().await {
		Ok(_) => assert!(true),
		Err(e) => panic!("Error starting workflows: {}", e),
	}

	Ok(())
}

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_tests_with_config_events_mpsc() -> pyo3::PyResult<()> {
	// create mpsc channel
	let (tx, mut rx) = tokio::sync::mpsc::channel(100);

	// Create a sample Config object
	let config = Config {
		version: 1.0,
		querent_id: "event_handler".to_string(),
		querent_name: "Test Querent event_handler".to_string(),
		workflow: WorkflowConfig {
			name: "test_workflow".to_string(),
			id: "workflow_id".to_string(),
			config: HashMap::new(),
			channel: None,
			inner_channel: Some(ChannelHandler::new()),
			inner_event_handler: Some(EventHandler::new(Some(tx))),
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

	// Create a WorkflowManager and add the Workflow

	let workflow_manager = WorkflowManager::new().expect("Failed to create WorkflowManager");
	assert!(workflow_manager.add_workflow(workflow).is_ok());

	// Start the workflows in a separate task
	tokio::spawn(async move {
		if let Err(e) = workflow_manager.start_workflows().await {
			log::error!("Error starting workflows: {}", e);
		}
	});
	// check if the event is received
	let event = rx.recv().await;
	println!("event is now: {:?}", event);
	assert!(event.is_some());
	let event = event.unwrap();
	assert_eq!(event.0, EventType::Graph);
	assert_eq!(event.1.timestamp, 123.45);
	assert_eq!(event.1.payload, "🚀🚀");

	Ok(())
}

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_tests_with_config_events_mpsc_separate_receiver(
) -> pyo3::PyResult<()> {
	// create mpsc channel
	let (tx, mut rx) = tokio::sync::mpsc::channel(100);

	// Create a sample Config object
	let config = Config {
		version: 1.0,
		querent_id: "event_handler".to_string(),
		querent_name: "Test Querent event_handler".to_string(),
		workflow: WorkflowConfig {
			name: "test_workflow".to_string(),
			id: "workflow_id".to_string(),
			config: HashMap::new(),
			channel: None,
			inner_channel: Some(ChannelHandler::new()),
			inner_event_handler: Some(EventHandler::new(Some(tx))),
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

	// Create a WorkflowManager and add the Workflow
	let workflow_manager = WorkflowManager::new().expect("Failed to create WorkflowManager");
	assert!(workflow_manager.add_workflow(workflow).is_ok());

	// Start the workflows in a separate task
	let workflow_task = tokio::spawn(async move {
		if let Err(e) = workflow_manager.start_workflows().await {
			log::error!("Error starting workflows: {}", e);
		}
	});

	// Start the receiver in another task
	let receiver_task = tokio::spawn(async move {
		// check if the event is received
		let event = rx.recv().await;
		println!("event is now in async: {:?}", event);
		assert!(event.is_some());
		let event = event.unwrap();
		assert_eq!(event.0, EventType::Graph);
		assert_eq!(event.1.timestamp, 123.45);
		assert_eq!(event.1.payload, "🚀🚀");
	});

	// Wait for both tasks to complete
	tokio::try_join!(workflow_task, receiver_task).map_err(|e| {
		log::error!("Error joining tasks: {}", e);
		PyTypeError::new_err("error message")
	})?;

	Ok(())
}

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_tests_with_config_events_qflow() -> pyo3::PyResult<()> {
	let quester = Quester::with_accelerated_time();
	let (event_streamer_messagebus, _indexer_inbox) = quester.create_test_messagebus();
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

	Ok(())
}

const CODE_CONFIG_CHANNEL_LOOP: &str = r#"
import asyncio

async def print_querent(config, text):
    """Prints the provided text and config"""
    print(text)
    channel = config['workflow']['channel']
    while True:
        result = channel.receive_in_python()
        print(result)
        if result is not None:
            break
"#;

#[pyo3_asyncio::tokio::test]
async fn workflow_manager_python_tests_with_config_channel_break() -> pyo3::PyResult<()> {
	// Create a sample Config object
	let config = Config {
		version: 1.0,
		querent_id: "event_handler".to_string(),
		querent_name: "Test Querent event_handler".to_string(),
		workflow: WorkflowConfig {
			name: "test_workflow".to_string(),
			id: "workflow_id".to_string(),
			config: HashMap::new(),
			channel: None,
			inner_channel: Some(ChannelHandler::new()),
			inner_event_handler: Some(EventHandler::new(None)),
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
		code: Some(CODE_CONFIG_CHANNEL_LOOP.to_string()),
		arguments: vec![CLRepr::String("Querent".to_string(), StringType::Normal)],
		config: Some(config),
	};

	// Create a WorkflowManager and add the Workflow
	let workflow_manager = WorkflowManager::new().expect("Failed to create WorkflowManager");
	assert!(workflow_manager.add_workflow(workflow).is_ok());

	// Start the workflows
	match workflow_manager.start_workflows().await {
		Ok(_) => assert!(true),
		Err(e) => panic!("Error starting workflows: {}", e),
	}

	Ok(())
}
