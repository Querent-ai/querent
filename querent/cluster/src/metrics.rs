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

use common::metrics::{new_counter, new_gauge, IntCounter, IntGauge};

#[allow(dead_code)]
pub struct GrpcClusterMetrics {
	pub live_nodes: IntGauge,
	pub ready_nodes: IntGauge,
	pub zombie_nodes: IntGauge,
	pub dead_nodes: IntGauge,
	pub cluster_state_size_bytes: IntGauge,
	pub node_state_size_bytes: IntGauge,
	pub node_state_keys: IntGauge,
	pub gossip_recv_messages_total: IntCounter,
	pub gossip_recv_bytes_total: IntCounter,
	pub gossip_sent_messages_total: IntCounter,
	pub gossip_sent_bytes_total: IntCounter,
	pub grpc_gossip_rounds_total: IntCounter,
}

impl Default for GrpcClusterMetrics {
	fn default() -> Self {
		GrpcClusterMetrics {
			live_nodes: new_gauge(
				"live_nodes",
				"The number of live nodes observed locally.",
				"cluster",
			),
			ready_nodes: new_gauge(
				"ready_nodes",
				"The number of ready nodes observed locally.",
				"cluster",
			),
			zombie_nodes: new_gauge(
				"zombie_nodes",
				"The number of zombie nodes observed locally.",
				"cluster",
			),
			dead_nodes: new_gauge(
				"dead_nodes",
				"The number of dead nodes observed locally.",
				"cluster",
			),
			cluster_state_size_bytes: new_gauge(
				"cluster_state_size_bytes",
				"The size of the cluster state in bytes.",
				"cluster",
			),
			node_state_keys: new_gauge(
				"node_state_keys",
				"The number of keys in the node state.",
				"cluster",
			),
			node_state_size_bytes: new_gauge(
				"node_state_size_bytes",
				"The size of the node state in bytes.",
				"cluster",
			),
			gossip_recv_messages_total: new_counter(
				"gossip_recv_messages_total",
				"Total number of gossip messages received.",
				"cluster",
			),
			gossip_recv_bytes_total: new_counter(
				"gossip_recv_bytes_total",
				"Total amount of gossip data received in bytes.",
				"cluster",
			),
			gossip_sent_messages_total: new_counter(
				"gossip_sent_messages_total",
				"Total number of gossip messages sent.",
				"cluster",
			),
			gossip_sent_bytes_total: new_counter(
				"gossip_sent_bytes_total",
				"Total amount of gossip data sent in bytes.",
				"cluster",
			),
			grpc_gossip_rounds_total: new_counter(
				"grpc_gossip_rounds_total",
				"Total number of gRPC gossip rounds performed with peer nodes.",
				"cluster",
			),
		}
	}
}
