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

use std::{fmt::Debug, net::SocketAddr, sync::Arc};

use chitchat::{ChitchatId, NodeState};
use tonic::transport::Channel;

use crate::{member::build_cluster_member, types::CpuCapacity};

#[derive(Clone)]
pub struct ClusterNode {
	inner: Arc<InnerNode>,
}

impl ClusterNode {
	/// Attempts to create a new `ClusterNode` from a Chitchat `NodeState`.
	pub(crate) fn try_new(
		chitchat_id: ChitchatId,
		node_state: &NodeState,
		channel: Channel,
		is_self_node: bool,
	) -> anyhow::Result<Self> {
		let member = build_cluster_member(chitchat_id.clone(), node_state)?;
		let inner = InnerNode {
			chitchat_id,
			channel,
			grpc_advertise_addr: member.grpc_advertise_addr,
			indexing_capacity: member.indexing_cpu_capacity,
			is_ready: member.is_ready,
			is_self_node,
		};
		let node = ClusterNode { inner: Arc::new(inner) };
		Ok(node)
	}

	pub fn chitchat_id(&self) -> &ChitchatId {
		&self.inner.chitchat_id
	}

	pub fn node_id(&self) -> &str {
		&self.inner.chitchat_id.node_id
	}

	pub fn channel(&self) -> Channel {
		self.inner.channel.clone()
	}

	pub fn grpc_advertise_addr(&self) -> SocketAddr {
		self.inner.grpc_advertise_addr
	}

	pub fn indexing_capacity(&self) -> CpuCapacity {
		self.inner.indexing_capacity
	}

	pub fn is_ready(&self) -> bool {
		self.inner.is_ready
	}

	pub fn is_self_node(&self) -> bool {
		self.inner.is_self_node
	}
}

impl Debug for ClusterNode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Node")
			.field("node_id", &self.inner.chitchat_id.node_id)
			.field("is_ready", &self.inner.is_ready)
			.finish()
	}
}

#[cfg(test)]
impl PartialEq for ClusterNode {
	fn eq(&self, other: &Self) -> bool {
		self.inner.chitchat_id == other.inner.chitchat_id &&
			self.inner.grpc_advertise_addr == other.inner.grpc_advertise_addr &&
			self.inner.is_ready == other.inner.is_ready &&
			self.inner.is_self_node == other.inner.is_self_node
	}
}

struct InnerNode {
	chitchat_id: ChitchatId,
	channel: Channel,
	grpc_advertise_addr: SocketAddr,
	indexing_capacity: CpuCapacity,
	is_ready: bool,
	is_self_node: bool,
}
