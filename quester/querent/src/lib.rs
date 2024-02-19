pub mod qsource;
use std::collections::HashMap;

use actors::{MessageBus, Quester};
use cluster::Cluster;
use common::{
	semantic_api::{NamedWorkflows, SemanticPipelineRequest, SupportedSources},
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
    print("🤖" + text) 
    querent_started = False

    try:
        import querent
        print("✨ Querent imported successfully ✨")
        querent_started = True
        await querent.workflow.start_workflow_engine(config)
        return
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
		.map(|c| {
			let config = match c.backend.clone() {
				SupportedSources::GCS(config) => {
					let mut map = HashMap::new();
					map.insert("bucket".to_string(), config.bucket);
					map.insert("credentials".to_string(), config.credentials);
					map
				},
				SupportedSources::S3(config) => {
					let mut map = HashMap::new();
					map.insert("bucket".to_string(), config.bucket);
					map.insert("region".to_string(), config.region);
					map.insert("access_key".to_string(), config.access_key);
					map.insert("secret_key".to_string(), config.secret_key);
					map
				},
				SupportedSources::AzureBlob(config) => {
					let mut map = HashMap::new();
					map.insert("container".to_string(), config.container);
					map.insert("connection_string".to_string(), config.connection_string);
					map.insert("account_url".to_string(), config.account_url);
					map.insert("credentials".to_string(), config.credentials);
					map.insert("prefix".to_string(), config.prefix.unwrap_or_default());
					map
				},
				SupportedSources::Drive(config) => {
					let mut map = HashMap::new();
					map.insert("drive_refresh_token".to_string(), config.drive_refresh_token);
					map.insert("drive_client_id".to_string(), config.drive_client_id);
					map.insert("drive_client_secret".to_string(), config.drive_client_secret);
					map.insert("drive_scopes".to_string(), config.drive_scopes);
					map.insert("drive_token".to_string(), config.drive_token);
					map.insert(
						"folder_to_crawl".to_string(),
						config.folder_to_crawl.unwrap_or_default(),
					);
					map.insert(
						"specific_file_type".to_string(),
						config.specific_file_type.unwrap_or_default(),
					);
					map
				},
				SupportedSources::Dropbox(config) => {
					let mut map = HashMap::new();
					map.insert("dropbox_app_key".to_string(), config.dropbox_app_key);
					map.insert("dropbox_app_secret".to_string(), config.dropbox_app_secret);
					map.insert("folder_path".to_string(), config.folder_path);
					map.insert("dropbox_refresh_token".to_string(), config.dropbox_refresh_token);
					map
				},
				SupportedSources::Github(config) => {
					let mut map = HashMap::new();
					map.insert("github_username".to_string(), config.github_username);
					map.insert("github_access_token".to_string(), config.github_access_token);
					map.insert("repository".to_string(), config.repository);
					map
				},
				SupportedSources::Slack(config) => {
					let mut map = HashMap::new();
					map.insert("access_token".to_string(), config.access_token);
					map.insert("channel_name".to_string(), config.channel_name);
					map.insert("cursor".to_string(), config.cursor.unwrap_or_default());
					map.insert(
						"include_all_metadata".to_string(),
						config.include_all_metadata.unwrap_or(true).to_string(),
					);
					map.insert("limit".to_string(), config.limit.unwrap_or(100).to_string());
					map
				},
				SupportedSources::Jira(config) => {
					let mut map = HashMap::new();
					map.insert("jira_server".to_string(), config.jira_server);
					map.insert("jira_username".to_string(), config.jira_username);
					map.insert(
						"jira_password".to_string(),
						config.jira_password.unwrap_or_default(),
					);
					map.insert("jira_project".to_string(), config.jira_project);
					map.insert("jira_query".to_string(), config.jira_query);
					map.insert(
						"jira_max_results".to_string(),
						config.jira_max_results.unwrap_or(100).to_string(),
					);
					map.insert(
						"jira_api_token".to_string(),
						config.jira_api_token.unwrap_or_default(),
					);
					map.insert(
						"jira_start_at".to_string(),
						config.jira_start_at.unwrap_or(0).to_string(),
					);
					map.insert("jira_keyfile".to_string(), config.jira_keyfile.unwrap_or_default());
					map.insert(
						"jira_certfile".to_string(),
						config.jira_certfile.unwrap_or_default(),
					);
					map.insert(
						"jira_verify".to_string(),
						config.jira_verify.unwrap_or_default().to_string(),
					);
					map
				},
				SupportedSources::Email(config) => {
					let mut map = HashMap::new();
					map.insert("imap_server".to_string(), config.imap_server);
					map.insert("imap_port".to_string(), config.imap_port.to_string());
					map.insert("imap_username".to_string(), config.imap_username);
					map.insert("imap_password".to_string(), config.imap_password);
					map.insert("imap_folder".to_string(), config.imap_folder);
					map.insert("imap_keyfile".to_string(), config.imap_keyfile.unwrap_or_default());
					map.insert(
						"imap_certfile".to_string(),
						config.imap_certfile.unwrap_or_default(),
					);
					map
				},
			};
			CollectorConfig {
				id: id.clone(),
				name: c.name.clone(),
				backend: c.backend.clone().to_string(),
				config,
				inner_channel: None,
				channel: None,
			}
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
