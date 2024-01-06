use std::{collections::HashMap, time::Duration};

use actors::{ActorExitStatus, ActorHandle, MessageBus, Quester};
use cluster::{start_cluster_service, Cluster};
use common::{BoxFutureInfaillible, NodeConfig, PubSubBroker, RuntimesConfig};
use querent::SemanticService;

const _READINESS_REPORTING_INTERVAL: Duration = if cfg!(any(test, feature = "testsuite")) {
	Duration::from_millis(25)
} else {
	Duration::from_secs(10)
};

pub struct QuesterServices {
	pub node_config: NodeConfig,
	pub cluster: Cluster,
	pub event_broker: PubSubBroker,
	pub quester_cloud: Quester,
	pub semantic_service_message_bus: MessageBus<SemanticService>,
	pub semantic_pipeline_handler: ActorHandle<SemanticService>,
}

pub async fn serve_quester(
	node_config: NodeConfig,
	_runtimes_config: RuntimesConfig,
	_shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<HashMap<String, ActorExitStatus>> {
	let _cluster = start_cluster_service(&node_config).await?;
	let _event_broker = PubSubBroker::default();
	let _quester_cloud = Quester::new();

	Err(anyhow::anyhow!("Not implemented"))
}
