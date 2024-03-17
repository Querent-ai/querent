use std::{
	collections::{BTreeMap, HashMap, HashSet},
	fmt::{Debug, Display},
	net::SocketAddr,
	sync::Arc,
	time::Duration,
};

use anyhow::Context;
use chitchat::{
	spawn_chitchat, transport::Transport, Chitchat, ChitchatConfig, ChitchatHandle, ChitchatId,
	ClusterStateSnapshot, FailureDetectorConfig, KeyChangeEvent, ListenerHandle, NodeState,
};
use futures::Stream;
use proto::semantics::IndexingStatistics;
use serde::{Deserialize, Serialize};
use tokio::{
	sync::{mpsc, watch, Mutex, RwLock},
	time::timeout,
};
use tokio_stream::{
	wrappers::{UnboundedReceiverStream, WatchStream},
	StreamExt,
};
use tracing::{info, warn};

use crate::{
	change::{compute_cluster_change_events, ClusterChange},
	member::{
		build_cluster_member, ClusterMember, NodeStateExt, GRPC_ADVERTISE_ADDR_KEY, READINESS_KEY,
		READINESS_VALUE_NOT_READY, READINESS_VALUE_READY, SEMANTIC_METRICS_PREFIX,
		SEMANTIC_PIPE_PREFIX,
	},
	types::NodeId,
	ClusterNode,
};

const GOSSIP_INTERVAL: Duration = if cfg!(any(test, feature = "testsuite")) {
	Duration::from_millis(25)
} else {
	Duration::from_secs(1)
};

const MARKED_FOR_DELETION_GRACE_PERIOD: usize = if cfg!(any(test, feature = "testsuite")) {
	100 // ~ HEARTBEAT * 100 = 2.5 seconds.
} else {
	5_000 // ~ HEARTBEAT * 5_000 ~ 4 hours.
};

#[derive(Clone)]
pub struct Cluster {
	cluster_id: String,
	self_chitchat_id: ChitchatId,
	/// Socket address (UDP) the node listens on for receiving gossip messages.
	pub gossip_listen_addr: SocketAddr,
	inner: Arc<RwLock<InnerCluster>>,
}

impl Debug for Cluster {
	fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter
			.debug_struct("Cluster")
			.field("cluster_id", &self.cluster_id)
			.field("self_node_id", &self.self_chitchat_id.node_id)
			.field("gossip_listen_addr", &self.gossip_listen_addr)
			.field("gossip_advertise_addr", &self.self_chitchat_id.gossip_advertise_addr)
			.finish()
	}
}

impl Cluster {
	pub fn cluster_id(&self) -> &str {
		&self.cluster_id
	}

	pub fn self_chitchat_id(&self) -> &ChitchatId {
		&self.self_chitchat_id
	}

	pub fn self_node_id(&self) -> &str {
		&self.self_chitchat_id.node_id
	}

	pub fn gossip_listen_addr(&self) -> SocketAddr {
		self.gossip_listen_addr
	}

	pub fn gossip_advertise_addr(&self) -> SocketAddr {
		self.self_chitchat_id.gossip_advertise_addr
	}

	pub async fn join(
		cluster_id: String,
		self_node: ClusterMember,
		gossip_listen_addr: SocketAddr,
		peer_seed_addrs: Vec<String>,
		failure_detector_config: FailureDetectorConfig,
		transport: &dyn Transport,
	) -> anyhow::Result<Self> {
		info!(
			cluster_id=%cluster_id,
			node_id=%self_node.node_id,
			gossip_listen_addr=%gossip_listen_addr,
			gossip_advertise_addr=%self_node.gossip_advertise_addr,
			grpc_advertise_addr=%self_node.grpc_advertise_addr,
			peer_seed_addrs=%peer_seed_addrs.join(", "),
			"Joining cluster."
		);
		let chitchat_config = ChitchatConfig {
			cluster_id: cluster_id.clone(),
			chitchat_id: self_node.chitchat_id(),
			listen_addr: gossip_listen_addr,
			seed_nodes: peer_seed_addrs,
			failure_detector_config,
			gossip_interval: GOSSIP_INTERVAL,
			marked_for_deletion_grace_period: MARKED_FOR_DELETION_GRACE_PERIOD,
		};
		let chitchat_handle = spawn_chitchat(
			chitchat_config,
			vec![
				(GRPC_ADVERTISE_ADDR_KEY.to_string(), self_node.grpc_advertise_addr.to_string()),
				(READINESS_KEY.to_string(), READINESS_VALUE_NOT_READY.to_string()),
			],
			transport,
		)
		.await?;

		let chitchat = chitchat_handle.chitchat();
		let live_nodes_stream = chitchat.lock().await.live_nodes_watcher();
		let (ready_members_tx, ready_members_rx) = watch::channel(Vec::new());

		spawn_ready_members_task(cluster_id.clone(), live_nodes_stream, ready_members_tx);
		let inner = InnerCluster {
			cluster_id: cluster_id.clone(),
			self_chitchat_id: self_node.chitchat_id(),
			chitchat_handle,
			live_nodes: BTreeMap::new(),
			change_stream_subscribers: Vec::new(),
			ready_members_rx,
		};
		let cluster = Cluster {
			cluster_id,
			self_chitchat_id: self_node.chitchat_id(),
			gossip_listen_addr,
			inner: Arc::new(RwLock::new(inner)),
		};
		spawn_ready_nodes_change_stream_task(cluster.clone()).await;
		Ok(cluster)
	}

	/// Deprecated: this is going away soon.
	pub async fn ready_members(&self) -> Vec<ClusterMember> {
		self.inner.read().await.ready_members_rx.borrow().clone()
	}

	/// Deprecated: this is going away soon.
	pub async fn ready_members_watcher(&self) -> WatchStream<Vec<ClusterMember>> {
		WatchStream::new(self.inner.read().await.ready_members_rx.clone())
	}

	/// Returns a stream of changes affecting the set of ready nodes in the cluster.
	pub async fn ready_nodes_change_stream(&self) -> impl Stream<Item = ClusterChange> {
		// The subscriber channel must be unbounded because we do no want to block when sending the
		// events.
		let (change_stream_tx, change_stream_rx) = mpsc::unbounded_channel();
		let mut inner = self.inner.write().await;
		for node in inner.live_nodes.values() {
			if node.is_ready() {
				change_stream_tx
					.send(ClusterChange::Add(node.clone()))
					.expect("The receiver end of the channel should be open.");
			}
		}
		inner.change_stream_subscribers.push(change_stream_tx);
		UnboundedReceiverStream::new(change_stream_rx)
	}

	/// Returns whether the self node is ready.
	pub async fn is_self_node_ready(&self) -> bool {
		self.chitchat()
			.await
			.lock()
			.await
			.node_state(&self.self_chitchat_id)
			.expect("The self node should always be present in the set of live nodes.")
			.is_ready()
	}

	/// Sets the self node's readiness.
	pub async fn set_self_node_readiness(&self, readiness: bool) {
		let readiness_value =
			if readiness { READINESS_VALUE_READY } else { READINESS_VALUE_NOT_READY };
		self.set_self_key_value(READINESS_KEY, readiness_value).await
	}

	/// Sets a key-value pair on the cluster node's state.
	pub async fn set_self_key_value(&self, key: impl Display, value: impl Display) {
		self.chitchat().await.lock().await.self_node_state().set(key, value);
	}

	pub async fn get_self_key_value(&self, key: &str) -> Option<String> {
		self.chitchat()
			.await
			.lock()
			.await
			.self_node_state()
			.get_versioned(key)
			.filter(|versioned_value| versioned_value.tombstone.is_none())
			.map(|versioned_value| versioned_value.value.clone())
	}

	pub async fn remove_self_key(&self, key: &str) {
		self.chitchat().await.lock().await.self_node_state().mark_for_deletion(key)
	}

	pub async fn subscribe(
		&self,
		key_prefix: &str,
		callback: impl Fn(KeyChangeEvent) + Send + Sync + 'static,
	) -> ListenerHandle {
		self.chitchat().await.lock().await.subscribe_event(key_prefix, callback)
	}

	/// Waits until the predicate holds true for the set of ready members.
	pub async fn wait_for_ready_members<F>(
		&self,
		mut predicate: F,
		timeout_after: Duration,
	) -> anyhow::Result<()>
	where
		F: FnMut(&[ClusterMember]) -> bool,
	{
		timeout(
			timeout_after,
			self.ready_members_watcher()
				.await
				.skip_while(|members| !predicate(members))
				.next(),
		)
		.await
		.context("deadline has passed before predicate held true")?;
		Ok(())
	}

	/// Returns a snapshot of the cluster state, including the underlying Chitchat state.
	pub async fn snapshot(&self) -> ClusterSnapshot {
		let chitchat = self.chitchat().await;
		let chitchat_guard = chitchat.lock().await;
		let chitchat_state_snapshot = chitchat_guard.state_snapshot();
		let mut ready_nodes = HashSet::new();
		let mut live_nodes = HashSet::new();

		for chitchat_id in chitchat_guard.live_nodes().cloned() {
			let node_state = chitchat_guard.node_state(&chitchat_id).expect(
				"The node should always be present in the cluster state because we hold the \
                 Chitchat mutex.",
			);
			if node_state.is_ready() {
				ready_nodes.insert(chitchat_id);
			} else {
				live_nodes.insert(chitchat_id);
			}
		}
		let dead_nodes = chitchat_guard.dead_nodes().cloned().collect::<HashSet<_>>();

		ClusterSnapshot {
			cluster_id: self.cluster_id.clone(),
			self_node_id: self.self_chitchat_id.node_id.clone(),
			ready_nodes,
			live_nodes,
			dead_nodes,
			chitchat_state_snapshot,
		}
	}

	/// Leaves the cluster.
	pub async fn shutdown(self) {
		info!(
			cluster_id=%self.cluster_id,
			node_id=%self.self_chitchat_id.node_id,
			"Leaving the cluster."
		);
		self.set_self_node_readiness(false).await;
		tokio::time::sleep(GOSSIP_INTERVAL * 2).await;
	}

	/// Value:      179m,76MB/s
	pub async fn update_semantic_service_metrics(
		&self,
		semantic_metrics: &HashMap<&String, IndexingStatistics>,
	) {
		let chitchat = self.chitchat().await;
		let mut chitchat_guard = chitchat.lock().await;
		let node_state = chitchat_guard.self_node_state();
		let mut current_metrics_keys: HashSet<String> = node_state
			.iter_prefix(SEMANTIC_METRICS_PREFIX)
			.map(|(key, _)| key.to_string())
			.collect();
		for (pipeline_id, metrics) in semantic_metrics {
			let key = format!("{SEMANTIC_METRICS_PREFIX}{pipeline_id}");
			current_metrics_keys.remove(&key);
			node_state.set(key, metrics.to_string());
		}
		for obsolete_task_key in current_metrics_keys {
			node_state.mark_for_deletion(&obsolete_task_key);
		}
	}

	pub async fn update_semantic_pipeline_state(
		&self,
		pipeline_id: &str,
		state: &str,
	) -> anyhow::Result<()> {
		let chitchat = self.chitchat().await;
		let mut chitchat_guard = chitchat.lock().await;
		let node_state = chitchat_guard.self_node_state();
		let key = format!("{SEMANTIC_PIPE_PREFIX}{pipeline_id}");
		node_state.set(key, state);
		Ok(())
	}

	pub async fn chitchat(&self) -> Arc<Mutex<Chitchat>> {
		self.inner.read().await.chitchat_handle.chitchat()
	}
}

/// Deprecated: this is going away soon.
fn spawn_ready_members_task(
	cluster_id: String,
	mut live_nodes_stream: WatchStream<BTreeMap<ChitchatId, NodeState>>,
	ready_members_tx: watch::Sender<Vec<ClusterMember>>,
) {
	let fut = async move {
		while let Some(new_live_nodes) = live_nodes_stream.next().await {
			let mut new_ready_members = Vec::with_capacity(new_live_nodes.len());

			for (chitchat_id, node_state) in new_live_nodes {
				let member = match build_cluster_member(chitchat_id, &node_state) {
					Ok(member) => member,
					Err(error) => {
						warn!(
							cluster_id=%cluster_id,
							error=?error,
							"Failed to build cluster member from Chitchat node state."
						);
						continue;
					},
				};
				if member.is_ready {
					new_ready_members.push(member);
				}
			}
			if *ready_members_tx.borrow() != new_ready_members &&
				ready_members_tx.send(new_ready_members).is_err()
			{
				break;
			}
		}
	};
	tokio::spawn(fut);
}

async fn spawn_ready_nodes_change_stream_task(cluster: Cluster) {
	let cluster_guard = cluster.inner.read().await;
	let cluster_id = cluster_guard.cluster_id.clone();
	let self_chitchat_id = cluster_guard.self_chitchat_id.clone();
	let chitchat = cluster_guard.chitchat_handle.chitchat();
	let weak_cluster = Arc::downgrade(&cluster.inner);
	drop(cluster_guard);
	drop(cluster);

	let mut previous_live_node_states = BTreeMap::new();
	let mut live_nodes_watcher = chitchat.lock().await.live_nodes_watcher();

	let future = async move {
		while let Some(new_live_node_states) = live_nodes_watcher.next().await {
			let Some(cluster) = weak_cluster.upgrade() else {
				break;
			};
			let mut cluster_guard = cluster.write().await;
			let previous_live_nodes = &mut cluster_guard.live_nodes;

			let events = compute_cluster_change_events(
				&cluster_id,
				&self_chitchat_id,
				previous_live_nodes,
				&previous_live_node_states,
				&new_live_node_states,
			)
			.await;
			if !events.is_empty() {
				cluster_guard.change_stream_subscribers.retain(|change_stream_tx| {
					events.iter().all(|event| change_stream_tx.send(event.clone()).is_ok())
				});
			}
			previous_live_node_states = new_live_node_states;
		}
	};
	tokio::spawn(future);
}

struct InnerCluster {
	cluster_id: String,
	self_chitchat_id: ChitchatId,
	chitchat_handle: ChitchatHandle,
	live_nodes: BTreeMap<NodeId, ClusterNode>,
	change_stream_subscribers: Vec<mpsc::UnboundedSender<ClusterChange>>,
	ready_members_rx: watch::Receiver<Vec<ClusterMember>>,
}

// Not used within the code, used for documentation.
#[derive(Debug, utoipa::ToSchema)]
pub struct NodeIdSchema {
	#[schema(example = "node-1")]
	/// The unique identifier of the node in the cluster.
	pub node_id: String,

	#[schema(example = "1683736537", value_type = u64)]
	/// A numeric identifier incremented every time the node leaves and rejoins the cluster.
	pub generation_id: u64,

	#[schema(example = "127.0.0.1:8000", value_type = String)]
	/// The socket address peers should use to gossip with the node.
	pub gossip_advertise_addr: SocketAddr,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ClusterSnapshot {
	#[schema(example = "cluster-1")]
	/// The ID of the cluster that the node is a part of.
	pub cluster_id: String,

	#[schema(value_type = NodeIdSchema)]
	/// The unique ID of the current node.
	pub self_node_id: String,

	#[schema(value_type  = Vec<NodeIdSchema>)]
	/// The set of cluster node IDs that are ready to handle requests.
	pub ready_nodes: HashSet<ChitchatId>,

	#[schema(value_type  = Vec<NodeIdSchema>)]
	/// The set of cluster node IDs that are alive but not ready.
	pub live_nodes: HashSet<ChitchatId>,

	#[schema(value_type  = Vec<NodeIdSchema>)]
	/// The set of cluster node IDs flagged as dead or faulty.
	pub dead_nodes: HashSet<ChitchatId>,

	#[schema(
        value_type = Object,
        example = json!({
            "key_values": {
                "grpc_advertise_addr": "127.0.0.1:8080",
                "enabled_services": "searcher",
            },
            "max_version": 5,
        })
    )]
	/// A complete snapshot of the Chitchat cluster state.
	pub chitchat_state_snapshot: ClusterStateSnapshot,
}
