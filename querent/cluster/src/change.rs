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

use std::collections::{btree_map::Entry, BTreeMap};
use chitchat::{ChitchatId, NodeState};
use common::{
	sorted_iter::{KeyDiff, SortedByKeyIterator},
	tower::{make_channel, warmup_channel},
};
use tonic::transport::Channel;
use tracing::{info, warn};

use crate::{member::NodeStateExt, types::NodeId, ClusterNode};

#[derive(Debug, Clone)]
pub enum ClusterChange {
	Add(ClusterNode),
	Update(ClusterNode),
	Remove(ClusterNode),
}

/// Compares the digests of the previous and new set of lives nodes, identifies the changes that
/// occurred in the cluster, and emits the corresponding events, focusing on ready nodes only.
pub(crate) async fn compute_cluster_change_events(
	cluster_id: &str,
	self_chitchat_id: &ChitchatId,
	previous_nodes: &mut BTreeMap<NodeId, ClusterNode>,
	previous_node_states: &BTreeMap<ChitchatId, NodeState>,
	new_node_states: &BTreeMap<ChitchatId, NodeState>,
) -> Vec<ClusterChange> {
	let mut cluster_events = Vec::new();

	for key_diff in previous_node_states.iter().diff_by_key(new_node_states.iter()) {
		match key_diff {
			// The node has joined the cluster.
			KeyDiff::Added(chitchat_id, node_state) => {
				let node_events = compute_cluster_change_events_on_added(
					cluster_id,
					self_chitchat_id,
					chitchat_id,
					node_state,
					previous_nodes,
				)
				.await;

				cluster_events.extend(node_events);
			},
			// The node's state has changed.
			KeyDiff::Unchanged(chitchat_id, previous_node_state, new_node_state)
				if previous_node_state.max_version() != new_node_state.max_version() =>
			{
				let node_event_opt = compute_cluster_change_events_on_updated(
					cluster_id,
					self_chitchat_id,
					chitchat_id,
					new_node_state,
					previous_nodes,
				)
				.await;

				if let Some(node_event) = node_event_opt {
					cluster_events.push(node_event);
				}
			},
			// The node's state has not changed.
			KeyDiff::Unchanged(_chitchat_id, _previous_max_version, _new_max_version) => {},
			// The node has left the cluster, i.e. it is considered dead by the failure detector.
			KeyDiff::Removed(chitchat_id, _node_state) => {
				let node_event_opt = compute_cluster_change_events_on_removed(
					cluster_id,
					self_chitchat_id,
					chitchat_id,
					previous_nodes,
				);

				if let Some(node_event) = node_event_opt {
					cluster_events.push(node_event);
				}
			},
		};
	}
	cluster_events
}

async fn compute_cluster_change_events_on_added(
	cluster_id: &str,
	self_chitchat_id: &ChitchatId,
	new_chitchat_id: &ChitchatId,
	new_node_state: &NodeState,
	previous_nodes: &mut BTreeMap<NodeId, ClusterNode>,
) -> Vec<ClusterChange> {
	let is_self_node = self_chitchat_id == new_chitchat_id;
	let new_node_id: NodeId = new_chitchat_id.node_id.clone().into();
	let maybe_previous_node_entry = previous_nodes.entry(new_node_id);

	let mut events = Vec::new();

	if let Entry::Occupied(previous_node_entry) = maybe_previous_node_entry {
		let previous_node_ref = previous_node_entry.get();

		if previous_node_ref.chitchat_id().generation_id > new_chitchat_id.generation_id {
			warn!(
				cluster_id=%cluster_id,
				rogue_node_id=%new_chitchat_id.node_id,
				rogue_node_ip=%new_chitchat_id.gossip_advertise_addr.ip(),
				"rogue node `{}` has rejoined the cluster with a lower incarnation ID and will be ignored",
				new_chitchat_id.node_id
			);
			return events;
		}
		info!(
			cluster_id=%cluster_id,
			node_id=%new_chitchat_id.node_id,
			"node `{}` has rejoined the cluster",
			new_chitchat_id.node_id
		);
		let previous_node = previous_node_entry.remove();

		if previous_node.is_ready() {
			events.push(ClusterChange::Remove(previous_node));
		}
	} else if !is_self_node {
		info!(
			cluster_id=%cluster_id,
			node_id=%new_chitchat_id.node_id,
			"node `{}` has joined the cluster",
			new_chitchat_id.node_id
		);
	}
	let Some(new_node) =
		try_new_node(cluster_id, new_chitchat_id, new_node_state, is_self_node).await
	else {
		return events;
	};
	let new_node_id: NodeId = new_node.node_id().into();
	previous_nodes.insert(new_node_id, new_node.clone());

	if new_node.is_ready() {
		if !is_self_node {
			info!(
				cluster_id=%cluster_id,
				node_id=%new_chitchat_id.node_id,
				"node `{}` has transitioned to ready state",
				new_chitchat_id.node_id
			);
		}
		warmup_channel(new_node.channel()).await;
		events.push(ClusterChange::Add(new_node));
	}
	events
}

async fn compute_cluster_change_events_on_updated(
	cluster_id: &str,
	self_chitchat_id: &ChitchatId,
	updated_chitchat_id: &ChitchatId,
	updated_node_state: &NodeState,
	previous_nodes: &mut BTreeMap<NodeId, ClusterNode>,
) -> Option<ClusterChange> {
	let previous_node = previous_nodes.get(&updated_chitchat_id.node_id)?.clone();
	let previous_channel = previous_node.channel();
	let is_self_node = self_chitchat_id == updated_chitchat_id;
	let updated_node = try_new_node_with_channel(
		cluster_id,
		updated_chitchat_id,
		updated_node_state,
		previous_channel,
		is_self_node,
	)?;
	let updated_node_id: NodeId = updated_node.chitchat_id().node_id.clone().into();
	previous_nodes.insert(updated_node_id, updated_node.clone());

	if !previous_node.is_ready() && updated_node.is_ready() {
		warmup_channel(updated_node.channel()).await;

		if !is_self_node {
			info!(
				cluster_id=%cluster_id,
				node_id=%updated_chitchat_id.node_id,
				"node `{}` has transitioned to ready state",
				updated_chitchat_id.node_id
			);
		}
		Some(ClusterChange::Add(updated_node))
	} else if previous_node.is_ready() && !updated_node.is_ready() {
		if !is_self_node {
			info!(
				cluster_id=%cluster_id,
				node_id=%updated_chitchat_id.node_id,
				"node `{}` has transitioned out of ready state",
				updated_chitchat_id.node_id
			);
		}
		Some(ClusterChange::Remove(updated_node))
	} else if previous_node.is_ready() && updated_node.is_ready() {
		Some(ClusterChange::Update(updated_node))
	} else {
		None
	}
}

fn compute_cluster_change_events_on_removed(
	cluster_id: &str,
	self_chitchat_id: &ChitchatId,
	removed_chitchat_id: &ChitchatId,
	previous_nodes: &mut BTreeMap<NodeId, ClusterNode>,
) -> Option<ClusterChange> {
	let removed_node_id: NodeId = removed_chitchat_id.node_id.clone().into();

	if let Entry::Occupied(previous_node_entry) = previous_nodes.entry(removed_node_id) {
		let previous_node_ref = previous_node_entry.get();

		if previous_node_ref.chitchat_id().generation_id == removed_chitchat_id.generation_id {
			if self_chitchat_id != removed_chitchat_id {
				info!(
					cluster_id=%cluster_id,
					node_id=%removed_chitchat_id.node_id,
					"node `{}` has left the cluster",
					removed_chitchat_id.node_id
				);
			}
			let previous_node = previous_node_entry.remove();

			if previous_node.is_ready() {
				return Some(ClusterChange::Remove(previous_node));
			}
		}
	};
	None
}

fn try_new_node_with_channel(
	cluster_id: &str,
	chitchat_id: &ChitchatId,
	node_state: &NodeState,
	channel: Channel,
	is_self_node: bool,
) -> Option<ClusterNode> {
	match ClusterNode::try_new(chitchat_id.clone(), node_state, channel, is_self_node) {
		Ok(node) => Some(node),
		Err(error) => {
			warn!(
				cluster_id=%cluster_id,
				node_id=%chitchat_id.node_id,
				error=%error,
				"failed to create cluster node from Chitchat node state"
			);
			None
		},
	}
}

async fn try_new_node(
	cluster_id: &str,
	chitchat_id: &ChitchatId,
	node_state: &NodeState,
	is_self_node: bool,
) -> Option<ClusterNode> {
	match node_state.grpc_advertise_addr() {
		Ok(socket_addr) => {
			let channel = make_channel(socket_addr).await;
			try_new_node_with_channel(cluster_id, chitchat_id, node_state, channel, is_self_node)
		},
		Err(error) => {
			warn!(
				cluster_id=%cluster_id,
				node_id=%chitchat_id.node_id,
				error=%error,
				"failed to read or parse gRPC advertise address"
			);
			None
		},
	}
}
