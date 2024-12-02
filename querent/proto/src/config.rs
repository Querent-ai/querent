use crate::semantics::StorageConfig;
use anyhow::{bail, ensure};
use bytesize::ByteSize;
use common::HostAddr;
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU64, time::Duration};
use tracing::warn;

pub const DEFAULT_CONFIG_PATH: &str = "config/querent.config.yaml";

pub const MB: u64 = 1_000_000;

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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StorageConfigs(pub Vec<StorageConfig>);

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
	pub rest_config: RestConfig,
	pub grpc_config: GrpcConfig,
	pub peer_seeds: Vec<String>,
	pub cpu_capacity: u32,
	pub memory_capacity: u32,
	pub storage_configs: StorageConfigs,
	pub tracing: Tracing,
}

impl Default for NodeConfig {
	fn default() -> Self {
		let rest_config = RestConfig {
			listen_port: 10074,
			cors_allow_origins: vec!["*".to_string()],
			extra_headers: HeaderMap::new(),
		};

		let grpc_config = GrpcConfig::default();
		Self {
			cluster_id: "querent-default".to_string(),
			node_id: "querent-default".to_string(),
			listen_address: "0.0.0.0".to_string(),
			advertise_address: "0.0.0.0".to_string(),
			gossip_listen_port: 10076,
			rest_config,
			grpc_config,
			peer_seeds: Vec::new(),
			cpu_capacity: 5,
			memory_capacity: 1000 * MB as u32,
			storage_configs: StorageConfigs(Vec::new()),
			tracing: Tracing { jaeger: JaegerConfig::default() },
		}
	}
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcConfig {
	pub listen_port: u16,
	#[serde(default = "GrpcConfig::default_max_message_size")]
	pub max_message_size: ByteSize,
}

impl GrpcConfig {
	fn default_max_message_size() -> ByteSize {
		ByteSize::mib(24)
	}

	pub fn validate(&self) -> anyhow::Result<()> {
		ensure!(
			self.max_message_size >= ByteSize::mb(1),
			"max gRPC message size (`grpc.max_message_size`) must be at least 1MB, got `{}`",
			self.max_message_size
		);
		Ok(())
	}
}

impl Default for GrpcConfig {
	fn default() -> Self {
		Self { max_message_size: Self::default_max_message_size(), listen_port: 10075 }
	}
}
