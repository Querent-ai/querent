use std::{path::PathBuf, sync::Arc};

use actors::{MessageBus, Quester};
use cluster::Cluster;
use common::PubSubBroker;
pub mod core;
pub use core::*;
use proto::{semantics::CollectorConfig, NodeConfig};
pub mod events;
pub use events::*;
pub mod storage;
pub use storage::*;
pub mod indexer;
pub mod pipeline;
pub use pipeline::*;
pub mod discovery;
pub use discovery::*;
pub mod ingest;
pub mod memory;
pub mod schemas;
pub mod tools;
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
    print(text + ": Engine Bot 🤖") 
    querent_started = False

    try:
        import querent
        print("✨ Querent imported successfully for ai engines✨")
        querent_started = True
        await querent.workflow.start_workflow(config)
        return
    except Exception as e:
        querent_started = False
        print("❌ Failed to import querent:  " + str(e))

"#;

pub async fn create_dynamic_sources(
	collectors_configs: Vec<CollectorConfig>,
) -> Result<Vec<Arc<dyn sources::Source>>, PipelineErrors> {
	let mut sources: Vec<Arc<dyn sources::Source>> = vec![];
	for collector in collectors_configs {
		match &collector.backend {
			Some(proto::semantics::Backend::Files(config)) => {
				let file_source = sources::filesystem::files::LocalFolderSource::new(
					PathBuf::from(config.root_path.clone()),
					None,
				);
				sources.push(Arc::new(file_source));
			},
			Some(proto::semantics::Backend::Gcs(config)) => {
				let gcs_source = sources::gcs::get_gcs_storage(config.clone()).map_err(|e| {
					PipelineErrors::InvalidParams(anyhow::anyhow!(
						"Failed to create GCS source: {}",
						e
					))
				})?;
				sources.push(Arc::new(gcs_source));
			},
			Some(proto::semantics::Backend::S3(config)) => {
				let s3_source = sources::s3::S3Source::new(config.clone()).await;
				sources.push(Arc::new(s3_source));
			},
			Some(proto::semantics::Backend::Azure(config)) => {
				let azure_source = sources::azure::AzureBlobStorage::new(config.clone());
				sources.push(Arc::new(azure_source));
			},
			_ =>
				return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
					"Invalid source type: {}",
					collector.name.clone(),
				))),
		};
	}
	Ok(sources)
}
