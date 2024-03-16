use std::net::SocketAddr;

use bytesize::ByteSize;
use common::tower::{make_channel, GrpcMetricsLayer};
use once_cell::sync::Lazy;
use proto::cluster::{
	cluster_service_grpc_server::ClusterServiceGrpcServer, ChitchatId as ProtoChitchatId,
	ClusterError, ClusterResult, ClusterService, ClusterServiceClient,
	ClusterServiceGrpcServerAdapter, FetchClusterStateRequest, FetchClusterStateResponse,
	NodeState as ProtoNodeState, VersionedKeyValue,
};
use tonic::async_trait;

use crate::Cluster;

const MAX_MESSAGE_SIZE: ByteSize = ByteSize::mib(64);

static CLUSTER_GRPC_CLIENT_METRICS_LAYER: Lazy<GrpcMetricsLayer> =
	Lazy::new(|| GrpcMetricsLayer::new("cluster", "client"));
static CLUSTER_GRPC_SERVER_METRICS_LAYER: Lazy<GrpcMetricsLayer> =
	Lazy::new(|| GrpcMetricsLayer::new("cluster", "server"));

pub(crate) async fn cluster_grpc_client(socket_addr: SocketAddr) -> ClusterServiceClient {
	let channel = make_channel(socket_addr).await;

	ClusterServiceClient::tower()
		.stack_layer(CLUSTER_GRPC_CLIENT_METRICS_LAYER.clone())
		.build_from_channel(socket_addr, channel, MAX_MESSAGE_SIZE)
}

pub fn cluster_grpc_server(
	cluster: Cluster,
) -> ClusterServiceGrpcServer<ClusterServiceGrpcServerAdapter> {
	ClusterServiceClient::tower()
		.stack_layer(CLUSTER_GRPC_SERVER_METRICS_LAYER.clone())
		.build(cluster)
		.as_grpc_service(MAX_MESSAGE_SIZE)
}

#[async_trait]
impl ClusterService for Cluster {
	async fn fetch_cluster_state(
		&mut self,
		request: FetchClusterStateRequest,
	) -> ClusterResult<FetchClusterStateResponse> {
		if request.cluster_id != self.cluster_id() {
			return Err(ClusterError::Internal("wrong cluster".to_string()));
		}
		let chitchat = self.chitchat().await;
		let chitchat_guard = chitchat.lock().await;

		let num_nodes = chitchat_guard.node_states().len();
		let mut proto_node_states = Vec::with_capacity(num_nodes);

		for (chitchat_id, node_state) in chitchat_guard.node_states() {
			let proto_chitchat_id = ProtoChitchatId {
				node_id: chitchat_id.node_id.clone(),
				generation_id: chitchat_id.generation_id,
				gossip_advertise_addr: chitchat_id.gossip_advertise_addr.to_string(),
			};

			let key_values: Vec<VersionedKeyValue> = node_state
				.key_values()
				.map(|(key, versioned_value)| VersionedKeyValue {
					key: key.to_string(),
					value: versioned_value.value.clone(),
					version: versioned_value.version,
					status: versioned_value.tombstone.unwrap_or(0) as i32,
				})
				.collect();
			if key_values.is_empty() {
				continue;
			}
			let proto_node_state = ProtoNodeState {
				chitchat_id: Some(proto_chitchat_id),
				key_values,
				max_version: node_state.max_version(),
				last_gc_version: node_state.max_version(),
			};
			proto_node_states.push(proto_node_state);
		}
		let response = FetchClusterStateResponse {
			cluster_id: request.cluster_id,
			node_states: proto_node_states,
		};
		Ok(response)
	}
}
