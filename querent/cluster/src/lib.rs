// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

#![deny(clippy::disallowed_methods)]

pub mod change;
pub mod cluster;
pub mod grpc_service;
pub mod member;
pub mod node;
pub mod types;
pub use grpc_service::*;
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
	let grpc_listen_addr = SocketAddr::new(listen_ip, node_config.grpc_config.listen_port);
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
