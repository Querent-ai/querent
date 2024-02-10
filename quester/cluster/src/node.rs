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
