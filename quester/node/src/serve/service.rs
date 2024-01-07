use std::{collections::HashMap, time::Duration};

use actors::{ActorExitStatus, MessageBus, Quester};
use cluster::{start_cluster_service, Cluster};
use common::{BoxFutureInfaillible, NodeConfig, PubSubBroker, RuntimesConfig};
use querent::{start_semantic_service, SemanticService};

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
	pub semantic_service_bus: MessageBus<SemanticService>,
}

pub async fn serve_quester(
	node_config: NodeConfig,
	_runtimes_config: RuntimesConfig,
	shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<HashMap<String, ActorExitStatus>> {
	let cluster = start_cluster_service(&node_config).await?;
	let event_broker = PubSubBroker::default();
	let quester_cloud = Quester::new();
	let _semantic_service_bus = start_semantic_service(
		&node_config,
		&quester_cloud,
		&cluster,
		&event_broker,
		shutdown_signal,
	)
	.await
	.expect("Failed to start semantic service");

	Err(anyhow::anyhow!("Not implemented"))
}
