use std::collections::HashMap;

use anyhow::bail;
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, EnumMap};
use tracing::warn;

use crate::HostAddr;

pub const DEFAULT_CONFIG_PATH: &str = "config/querent.config.yaml";

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub enum StorageType {
	#[serde(rename = "index")]
	Index,
	#[serde(rename = "graph")]
	Graph,
	#[serde(rename = "vector")]
	Vector,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum StorageConfig {
	#[serde(rename = "postgres")]
	Postgres(StorageBackend),
	#[serde(rename = "milvus")]
	Milvus(StorageBackend),
	#[serde(rename = "neo4j")]
	Neo4j(StorageBackend),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageBackend {
	pub name: String,
	pub storage_type: StorageType,
	pub config: HashMap<String, String>,
}

#[serde_as]
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct StorageConfigs(#[serde_as(as = "EnumMap")] Vec<StorageConfig>);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfig {
	pub cluster_id: String,
	pub node_id: String,
	pub listen_address: String,
	pub advertise_address: String,
	pub gossip_listen_port: u16,
	pub grpc_listen_port: u16,
	pub rest_config: RestConfig,
	pub peer_seeds: Vec<String>,
	pub cpu_capacity: u32,
	pub storage_configs: StorageConfigs,
}

impl NodeConfig {
	/// Returns the list of peer seed addresses. The addresses MUST NOT be resolved. Otherwise, the
	/// DNS-based discovery mechanism implemented in Chitchat will not work correctly.
	pub async fn peer_seed_addrs(&self) -> anyhow::Result<Vec<String>> {
		let mut peer_seed_addrs = Vec::new();
		let default_gossip_port = self.gossip_listen_port;

		// We want to pass non-resolved addresses to Chitchat but still want to resolve them for
		// validation purposes. Additionally, we need to append a default port if necessary and
		// finally return the addresses as strings, which is tricky for IPv6. We let the logic baked
		// in `HostAddr` handle this complexity.
		for peer_seed in &self.peer_seeds {
			let peer_seed_addr = HostAddr::parse_with_default_port(peer_seed, default_gossip_port)?;
			if let Err(error) = peer_seed_addr.resolve().await {
				warn!(peer_seed = %peer_seed_addr, error = ?error, "failed to resolve peer seed address");
				continue;
			}
			peer_seed_addrs.push(peer_seed_addr.to_string())
		}
		if !self.peer_seeds.is_empty() && peer_seed_addrs.is_empty() {
			bail!(
				"failed to resolve any of the peer seed addresses: `{}`",
				self.peer_seeds.join(", ")
			)
		}
		Ok(peer_seed_addrs)
	}
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestConfig {
	pub listen_port: u16,
	pub cors_allow_origins: Vec<String>,
	#[serde(with = "http_serde::header_map")]
	pub extra_headers: HeaderMap,
}
