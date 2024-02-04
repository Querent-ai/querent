pub mod qsource;
use std::collections::HashMap;

use actors::{MessageBus, Quester};
use cluster::Cluster;
use common::{
	semantic_api::{NamedWorkflows, SemanticPipelineRequest},
	NodeConfig, PubSubBroker,
};
pub use qsource::*;
pub mod events;
pub use events::*;
pub mod source;
use querent_synapse::{
	config::{
		config::{CollectorConfig, EngineConfig, WorkflowConfig},
		Config,
	},
	cross::{CLRepr, StringType},
	querent::Workflow,
};
pub use source::*;
pub mod storage;
pub use storage::*;
pub mod indexer;
pub mod pipeline;
pub use pipeline::*;
use tracing::info;

#[allow(clippy::too_many_arguments)]
pub async fn start_semantic_service(
	node_config: &NodeConfig,
	quester: &Quester,
	cluster: &Cluster,
	pubsub_broker: &PubSubBroker,
) -> anyhow::Result<MessageBus<SemanticService>> {
	info!("Starting semantic service");

	let semantic_service =
		SemanticService::new(node_config.node_id.clone(), cluster.clone(), pubsub_broker.clone());

	let (semantic_service_mailbox, _) = quester.spawn_builder().spawn(semantic_service);
	info!("Starting semantic service started");
	Ok(semantic_service_mailbox)
}

const _CODE_CONFIG_EVENT_HANDLER: &str = r#"
import asyncio
import json

async def print_querent(config, text: str):
    print( "🤖" + text) 
    querent_started = False

    try:
        import querent
        print("✨ Querent imported successfully ✨")
        querent_started = True
        await querent.workflow.start_workflow(config)
    except Exception as e:
        querent_started = False
        print("❌ Failed to import querent: " + str(e))

    while True and not querent_started:
        print("⌛ Waiting for querent to start...sending dummy events")
        message_state = config['workflow']['channel'].receive_in_python()
        tokens_received = config['workflow']['tokens_feader'].receive_tokens_in_python()

        if tokens_received is not None:
            print("📜 Received tokens: " + str(tokens_received['data']))

        if message_state is not None:
            message_type = message_state['message_type']

            if message_type.lower() == "stop":
                print("🛑 Received stop signal. Exiting...")
                break
            else:
                print("📬 Received message of type: " + message_type)
                # Handle other message types...

        # Continue sending events
        if config['workflow'] is not None:
            event_type = "Graph"  # Replace with the desired event type
            payload = {
                "subject": "Querent AI LLC",
                "subject_type": "Organization",
                "object": "Querent",
                "object_type": "Software",
                "predicate": "developed by",
                "predicate_type": "ownership",
                "sentence": "Querent is developed by Querent AI LLC"
            }
            event_data = {
                "event_type": event_type,
                "timestamp": 123.45,  # Replace with the actual timestamp
                "payload": json.dumps(payload),
                "file": "file_name"  # Replace with the actual file name
            }
            config['workflow']['event_handler'].handle_event(event_type, event_data)

        await asyncio.sleep(1)  # Adjust the sleep duration as needed
"#;

pub async fn create_querent_synapose_workflow(
	id: String,
	request: &SemanticPipelineRequest,
) -> Result<Workflow, PipelineErrors> {
	let collector_configs: Vec<CollectorConfig> = request
		.collectors
		.iter()
		.map(|c| CollectorConfig {
			id: id.clone(),
			name: c.name.clone(),
			backend: c.backend.clone().into(),
			inner_channel: None,
			channel: None,
			config: c.config.clone(),
		})
		.collect();
	let mut engine_additionl_config = HashMap::new();
	match request.name {
		NamedWorkflows::KnowledgeGraphUsingOpenAI(ref config) => {
			engine_additionl_config
				.insert("openai_api_key".to_string(), config.openai_api_key.clone());
		},
		_ => {},
	}
	let engine_configs: Vec<EngineConfig> = vec![EngineConfig {
		id: id.clone(),
		name: request.name.clone().into(),
		inner_channel: None,
		channel: None,
		config: engine_additionl_config,
	}];
	let config = Config {
		version: request.version.clone(),
		querent_id: id.to_string(),
		querent_name: request.name.clone().into(),
		workflow: WorkflowConfig {
			name: request.name.clone().into(),
			id: id.to_string(),
			config: request.config.clone(),
			channel: None,
			inner_channel: None,
			inner_event_handler: None,
			event_handler: None,
			inner_tokens_feader: None,
			tokens_feader: None,
		},
		collectors: collector_configs,
		engines: engine_configs,
		resource: None,
	};
	let workflow = Workflow {
		name: request.name.clone().into(),
		id: id.to_string(),
		import: "".to_string(),
		attr: "print_querent".to_string(),
		code: Some(_CODE_CONFIG_EVENT_HANDLER.to_string()),
		arguments: vec![CLRepr::String("Starting Querent".to_string(), StringType::Normal)],
		config: Some(config),
	};
	Ok(workflow)
}
