use std::{num::NonZeroU64, time::Duration};

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub enum StorageConfig {
	#[serde(rename = "postgres")]
	Postgres(PostgresConfig),
	#[serde(rename = "milvus")]
	Milvus(MilvusConfig),
	#[serde(rename = "neo4j")]
	Neo4j(Neo4jConfig),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PostgresConfig {
	pub name: String,
	pub storage_type: StorageType,
	pub url: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MilvusConfig {
	pub name: String,
	pub storage_type: StorageType,
	pub url: String,
	pub username: String,
	pub password: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Neo4jConfig {
	pub name: String,
	pub storage_type: StorageType,
	pub db_name: String,
	pub url: String,
	pub username: String,
	pub password: String,
	pub max_connection_pool_size: Option<usize>,
	pub fetch_size: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JaegerConfig {
	/// Enables the gRPC endpoint that allows the Jaeger Query Service to connect and retrieve
	/// traces.
	#[serde(default = "JaegerConfig::default_enable_endpoint")]
	pub enable_endpoint: bool,
	/// How far back in time we look for spans when queries at not time-bound (`get_services`,
	/// `get_operations`, `get_trace` operations).
	#[serde(default = "JaegerConfig::default_lookback_period_hours")]
	lookback_period_hours: NonZeroU64,
	/// The assumed maximum duration of a trace in seconds.
	///
	/// Finding a trace happens in two phases: the first phase identifies at least one span that
	/// matches the query, while the second phase retrieves the spans that belong to the trace.
	/// The `max_trace_duration_secs` parameter is used during the second phase to restrict the
	/// search time interval to [span.end_timestamp - max_trace_duration, span.start_timestamp
	/// + max_trace_duration].
	#[serde(default = "JaegerConfig::default_max_trace_duration_secs")]
	max_trace_duration_secs: NonZeroU64,
	/// The maximum number of spans that can be retrieved in a single request.
	#[serde(default = "JaegerConfig::default_max_fetch_spans")]
	pub max_fetch_spans: NonZeroU64,
}

impl JaegerConfig {
	pub fn lookback_period(&self) -> Duration {
		Duration::from_secs(self.lookback_period_hours.get() * 3600)
	}

	pub fn max_trace_duration(&self) -> Duration {
		Duration::from_secs(self.max_trace_duration_secs.get())
	}

	fn default_enable_endpoint() -> bool {
		#[cfg(any(test, feature = "testsuite"))]
		{
			false
		}
		#[cfg(not(any(test, feature = "testsuite")))]
		{
			true
		}
	}

	fn default_lookback_period_hours() -> NonZeroU64 {
		NonZeroU64::new(72).unwrap() // 3 days
	}

	fn default_max_trace_duration_secs() -> NonZeroU64 {
		NonZeroU64::new(3600).unwrap() // 1 hour
	}

	fn default_max_fetch_spans() -> NonZeroU64 {
		NonZeroU64::new(10_000).unwrap() // 10k spans
	}
}

impl Default for JaegerConfig {
	fn default() -> Self {
		Self {
			enable_endpoint: Self::default_enable_endpoint(),
			lookback_period_hours: Self::default_lookback_period_hours(),
			max_trace_duration_secs: Self::default_max_trace_duration_secs(),
			max_fetch_spans: Self::default_max_fetch_spans(),
		}
	}
}

#[serde_as]
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StorageConfigs(#[serde_as(as = "EnumMap")] pub Vec<StorageConfig>);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tracing {
	pub jaeger: JaegerConfig,
}
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
	pub tracing: Tracing,
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
