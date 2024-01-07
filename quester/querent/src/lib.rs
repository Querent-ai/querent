pub mod qsource;
use actors::{MessageBus, Quester};
use cluster::Cluster;
use common::{NodeConfig, PubSubBroker};
pub use qsource::*;
pub mod events;
pub use events::*;
pub mod source;
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
	Ok(semantic_service_mailbox)
}
