use std::net::SocketAddr;

use anyhow::bail;
use http::HeaderMap;
use querent_synapse::config::Config;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{HostAddr, StorageConfig};

#[derive(Debug, Clone)]
pub struct QuesterConfig {
	/// The id of the qflow, a single qflow pipeline runs a single qflow.
	pub qflow_id: String,
	/// The configuration for Querent.
	pub qflow_config: Config,
	/// The Storage configuration for Quester
	pub storage_config: StorageConfig,
}

impl Default for QuesterConfig {
	fn default() -> Self {
		Self {
			qflow_id: "qflow".to_string(),
			qflow_config: Config::default(),
			storage_config: StorageConfig::Postgres(crate::PostgresConfig {
				url: "postgresql://user:password@host:port/database".to_string(),
			}),
		}
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfig {
	pub cluster_id: String,
	pub node_id: String,
	pub gossip_listen_addr: SocketAddr,
	pub grpc_listen_addr: SocketAddr,
	pub gossip_advertise_addr: SocketAddr,
	pub grpc_advertise_addr: SocketAddr,
	pub rest_config: RestConfig,
	pub peer_seeds: Vec<String>,
	pub cpu_capacity: u32,
}

impl NodeConfig {
	/// Returns the list of peer seed addresses. The addresses MUST NOT be resolved. Otherwise, the
	/// DNS-based discovery mechanism implemented in Chitchat will not work correctly.
	pub async fn peer_seed_addrs(&self) -> anyhow::Result<Vec<String>> {
		let mut peer_seed_addrs = Vec::new();
		let default_gossip_port = self.gossip_listen_addr.port();

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
	pub listen_addr: SocketAddr,
	pub cors_allow_origins: Vec<String>,
	#[serde(with = "http_serde::header_map")]
	pub extra_headers: HeaderMap,
}
