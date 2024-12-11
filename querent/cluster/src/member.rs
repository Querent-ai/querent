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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use std::{net::SocketAddr, str::FromStr};
use anyhow::Context;
use chitchat::{ChitchatId, NodeState};
use tracing::error;

use crate::{
	types::{CpuCapacity, NodeId},
	GenerationId,
};

// Keys used to store member's data in chitchat state.
pub(crate) const GRPC_ADVERTISE_ADDR_KEY: &str = "grpc_advertise_addr";
// Readiness key and values used to store node's readiness in Chitchat state.
pub(crate) const READINESS_KEY: &str = "readiness";
pub(crate) const READINESS_VALUE_READY: &str = "READY";
pub(crate) const READINESS_VALUE_NOT_READY: &str = "NOT_READY";
pub(crate) const SEMANTIC_METRICS_PREFIX: &str = "semantic_metrics:";
pub(crate) const SEMANTIC_PIPE_PREFIX: &str = "semantic_pipelines:";

pub const SEMANTIC_CPU_CAPACITY_KEY: &str = "semantic_cpu_capacity";

pub(crate) trait NodeStateExt {
	fn grpc_advertise_addr(&self) -> anyhow::Result<SocketAddr>;

	fn is_ready(&self) -> bool;
}

impl NodeStateExt for NodeState {
	fn grpc_advertise_addr(&self) -> anyhow::Result<SocketAddr> {
		self.get(GRPC_ADVERTISE_ADDR_KEY)
			.with_context(|| {
				format!("could not find key `{GRPC_ADVERTISE_ADDR_KEY}` in Chitchat node state")
			})
			.map(|grpc_advertise_addr_value| {
				grpc_advertise_addr_value.parse().with_context(|| {
					format!("failed to parse gRPC advertise address `{grpc_advertise_addr_value}`")
				})
			})?
	}

	fn is_ready(&self) -> bool {
		self.get(READINESS_KEY)
			.map(|health_value| health_value == READINESS_VALUE_READY)
			.unwrap_or(false)
	}
}

/// Cluster member.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClusterMember {
	/// A unique node ID across the cluster.
	/// The Chitchat node ID is the concatenation of the node ID and the start timestamp:
	/// `{node_id}/{start_timestamp}`.
	pub node_id: NodeId,
	/// The start timestamp (seconds) of the node.
	pub generation_id: GenerationId,
	/// Gossip advertise address, i.e. the address that other nodes should use to gossip with the
	/// node.
	pub gossip_advertise_addr: SocketAddr,
	/// gRPC advertise address, i.e. the address that other nodes should use to communicate with
	/// the node via gRPC.
	pub grpc_advertise_addr: SocketAddr,
	/// Indexing cpu capacity of the node expressed in milli cpu.
	pub indexing_cpu_capacity: CpuCapacity,
	pub is_ready: bool,
}

impl ClusterMember {
	pub fn chitchat_id(&self) -> ChitchatId {
		ChitchatId::new(
			self.node_id.clone().into(),
			self.generation_id.as_u64(),
			self.gossip_advertise_addr,
		)
	}
}

impl From<ClusterMember> for ChitchatId {
	fn from(member: ClusterMember) -> Self {
		member.chitchat_id()
	}
}

fn parse_indexing_cpu_capacity(node_state: &NodeState) -> CpuCapacity {
	let Some(indexing_capacity_str) = node_state.get(SEMANTIC_CPU_CAPACITY_KEY) else {
		return CpuCapacity::zero();
	};
	if let Ok(indexing_capacity) = CpuCapacity::from_str(indexing_capacity_str) {
		indexing_capacity
	} else {
		error!(indexing_capacity=?indexing_capacity_str, "received an unparseable indexing capacity from node");
		CpuCapacity::zero()
	}
}

// Builds a cluster member from a [`NodeState`].
pub(crate) fn build_cluster_member(
	chitchat_id: ChitchatId,
	node_state: &NodeState,
) -> anyhow::Result<ClusterMember> {
	let is_ready = node_state.is_ready();
	let grpc_advertise_addr = node_state.grpc_advertise_addr()?;
	let indexing_cpu_capacity = parse_indexing_cpu_capacity(node_state);
	let member = ClusterMember {
		node_id: chitchat_id.node_id.into(),
		generation_id: chitchat_id.generation_id.into(),
		is_ready,
		gossip_advertise_addr: chitchat_id.gossip_advertise_addr,
		grpc_advertise_addr,
		indexing_cpu_capacity,
	};
	Ok(member)
}
