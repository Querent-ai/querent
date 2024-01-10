pub mod qsource;
use actors::{MessageBus, Quester};
use cluster::Cluster;
use common::{NodeConfig, PubSubBroker, SemanticPipelineRequest};
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

pub async fn create_querent_synapose_workflow(
	id: String,
	request: &SemanticPipelineRequest,
) -> Result<Workflow, PipelineErrors> {
	let collector_configs: Vec<CollectorConfig> = request
		.workflow_config
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
	let engine_configs: Vec<EngineConfig> = request
		.workflow_config
		.engines
		.iter()
		.map(|c| EngineConfig {
			id: id.clone(),
			name: c.backend.clone().into(),
			inner_channel: None,
			channel: None,
			config: c.config.clone(),
		})
		.collect();
	let config = Config {
		version: request.workflow_config.version.clone(),
		querent_id: id.to_string(),
		querent_name: request.name.clone(),
		workflow: WorkflowConfig {
			name: request.name.clone(),
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
		name: request.name.clone(),
		id: id.to_string(),
		import: request.import.clone(),
		attr: request.attr.clone(),
		code: request.code.clone(),
		arguments: vec![CLRepr::String("Starting Querent".to_string(), StringType::Normal)],
		config: Some(config),
	};
	Ok(workflow)
}
