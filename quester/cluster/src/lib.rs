#![deny(clippy::disallowed_methods)]

pub mod change;
pub mod cluster;
pub mod member;
pub mod node;
pub mod types;
use proto::NodeConfig;
mod metrics;

use std::net::SocketAddr;

use chitchat::transport::UdpTransport;
pub use chitchat::{
	transport::ChannelTransport, FailureDetectorConfig, KeyChangeEvent, ListenerHandle,
};
use common::Host;
use time::OffsetDateTime;

pub use crate::{
	change::ClusterChange,
	cluster::{Cluster, ClusterSnapshot, NodeIdSchema},
	member::{ClusterMember, SEMANTIC_CPU_CAPACITY_KEY},
	node::ClusterNode,
};

use self::types::{CpuCapacity, NodeId};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct GenerationId(u64);

impl GenerationId {
	pub fn as_u64(&self) -> u64 {
		self.0
	}

	pub fn now() -> Self {
		Self(OffsetDateTime::now_utc().unix_timestamp_nanos() as u64)
	}
}

impl From<u64> for GenerationId {
	fn from(generation_id: u64) -> Self {
		Self(generation_id)
	}
}

pub async fn start_cluster_service(node_config: &NodeConfig) -> anyhow::Result<Cluster> {
	let cluster_id = node_config.cluster_id.clone();
	let listen_host = node_config.listen_address.parse::<Host>()?;
	let listen_ip = listen_host.resolve().await?;

	let gossip_listen_addr = SocketAddr::new(listen_ip, node_config.gossip_listen_port);
	let grpc_listen_addr = SocketAddr::new(listen_ip, node_config.grpc_listen_port);
	let peer_seed_addrs = node_config.peer_seed_addrs().await?;

	let node_id: NodeId = node_config.node_id.clone().into();
	let generation_id = GenerationId::now();
	let is_ready = false;
	let cpu_capacity = CpuCapacity::from_cpu_millis(node_config.cpu_capacity);
	let self_node = ClusterMember {
		node_id,
		generation_id,
		is_ready,
		gossip_advertise_addr: gossip_listen_addr,
		grpc_advertise_addr: grpc_listen_addr,
		indexing_cpu_capacity: cpu_capacity,
	};
	let cluster: Cluster = Cluster::join(
		cluster_id,
		self_node,
		gossip_listen_addr,
		peer_seed_addrs,
		FailureDetectorConfig::default(),
		&UdpTransport,
	)
	.await?;
	cluster.set_self_key_value(SEMANTIC_CPU_CAPACITY_KEY, cpu_capacity).await;

	Ok(cluster)
}
