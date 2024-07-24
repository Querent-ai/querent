use std::{path::PathBuf, sync::Arc};

use actors::{MessageBus, Querent};
use cluster::Cluster;
use common::PubSubBroker;
use once_cell::sync::Lazy;
use tokio::sync::Semaphore;
pub mod core;
pub use core::*;
use proto::{semantics::CollectorConfig, DiscoveryAgentType, NodeConfig};
pub mod events;
pub use events::*;
pub mod storage;
pub use storage::*;
pub mod indexer;
pub mod pipeline;
pub use pipeline::*;
pub mod discovery;
pub use discovery::*;
pub mod insights;
pub use insights::*;
pub mod ingest;
use serde::{Deserialize, Serialize};
use sp_core::sr25519;
use sp_runtime::{traits::Verify, MultiSignature};
use tracing::info;

pub static INGESTOR_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(10));

#[allow(clippy::too_many_arguments)]
pub async fn start_semantic_service(
	node_config: &NodeConfig,
	querent: &Querent,
	cluster: &Cluster,
	pubsub_broker: &PubSubBroker,
	secret_store: Arc<dyn storage::Storage>,
) -> anyhow::Result<MessageBus<SemanticService>> {
	info!("Starting semantic service");

	let semantic_service = SemanticService::new(
		node_config.node_id.clone(),
		cluster.clone(),
		pubsub_broker.clone(),
		secret_store,
	);

	let (semantic_service_mailbox, _) = querent.spawn_builder().spawn(semantic_service);
	info!("Starting semantic service started");
	Ok(semantic_service_mailbox)
}

pub async fn create_dynamic_sources(
	licence_key: Option<String>,
	collectors_configs: Vec<CollectorConfig>,
) -> Result<Vec<Arc<dyn sources::Source>>, PipelineErrors> {
	let mut sources: Vec<Arc<dyn sources::Source>> = vec![];
	let licence_key = licence_key.unwrap_or_default();
	if licence_key.is_empty() {
		log::warn!("Missing License Key");
	}
	for collector in collectors_configs {
		match &collector.backend {
			Some(proto::semantics::Backend::Files(config)) => {
				#[cfg(feature = "license-check")]
				if !is_data_source_allowed_by_product(licence_key.clone(), &collector).unwrap() {
					return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
						"Data source not allowed by product: {}",
						collector.name.clone(),
					)));
				}
				let file_source = sources::filesystem::files::LocalFolderSource::new(
					PathBuf::from(config.root_path.clone()),
					None,
					config.id.clone(),
				);
				sources.push(Arc::new(file_source));
			},
			Some(proto::semantics::Backend::Gcs(config)) => {
				#[cfg(feature = "license-check")]
				if !is_data_source_allowed_by_product(licence_key.clone(), &collector).unwrap() {
					return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
						"Data source not allowed by product: {}",
						collector.name.clone(),
					)));
				}
				let gcs_source = sources::gcs::get_gcs_storage(config.clone()).map_err(|e| {
					PipelineErrors::InvalidParams(anyhow::anyhow!(
						"Failed to create GCS source: {}",
						e
					))
				})?;
				sources.push(Arc::new(gcs_source));
			},
			Some(proto::semantics::Backend::S3(config)) => {
				#[cfg(feature = "license-check")]
				if !is_data_source_allowed_by_product(licence_key.clone(), &collector).unwrap() {
					return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
						"Data source not allowed by product: {}",
						collector.name.clone(),
					)));
				}
				let s3_source = sources::s3::S3Source::new(config.clone()).await;
				sources.push(Arc::new(s3_source));
			},
			Some(proto::semantics::Backend::Azure(config)) => {
				#[cfg(feature = "license-check")]
				if !is_data_source_allowed_by_product(licence_key.clone(), &collector).unwrap() {
					return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
						"Data source not allowed by product: {}",
						collector.name.clone(),
					)));
				}
				let azure_source = sources::azure::AzureBlobStorage::new(config.clone());
				sources.push(Arc::new(azure_source));
			},
			Some(proto::semantics::Backend::Drive(config)) => {
				let drive_source =
					sources::drive::drive::GoogleDriveSource::new(config.clone()).await;
				sources.push(Arc::new(drive_source));
			},
			Some(proto::semantics::Backend::Onedrive(config)) =>
				match sources::onedrive::onedrive::OneDriveSource::new(config.clone()).await {
					Ok(onedrive_source) => {
						sources.push(Arc::new(onedrive_source));
					},
					Err(e) => {
						return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
							"Failed to initialize email source: {:?} ",
							e
						)));
					},
				},
			Some(proto::semantics::Backend::Email(config)) =>
				match sources::email::email::EmailSource::new(config.clone()).await {
					Ok(email_source) => {
						sources.push(Arc::new(email_source) as Arc<dyn sources::Source>);
					},
					Err(e) => {
						return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
							"Failed to initialize email source: {:?} ",
							e
						)));
					},
				},
			_ =>
				return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
					"Invalid source type: {}",
					collector.name.clone(),
				))),
		};
	}
	Ok(sources)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPayload {
	pub payload: ProductRegistrationInfo,
	pub signature: MultiSignature,
	#[serde(rename = "publicKey")]
	pub public_key: String,
	pub expiry: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRegistrationInfo {
	pub name: String,
	pub website: String,
	pub email: String,
	pub product: ProductType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProductType {
	#[serde(rename = "rian")]
	Rian,
	#[serde(rename = "rianpro")]
	RianPro,
	#[serde(rename = "rianenterprise")]
	RianEnterprise,
}

impl ProductType {
	pub fn to_string(&self) -> String {
		match self {
			ProductType::Rian => "rian".to_string(),
			ProductType::RianPro => "rianpro".to_string(),
			ProductType::RianEnterprise => "rianenterprise".to_string(),
		}
	}
}

/// Verify a license key
#[allow(deprecated)]
pub fn verify_key(licence_key: String) -> Result<bool, anyhow::Error> {
	// license_key is a base64 encoded string
	let key = base64::decode(licence_key)?;
	// parse key into ProductRegistrationInfo
	let product_sign: SignedPayload = serde_json::from_slice(&key)?;
	// Check the expiry is not in the past
	let now = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	if product_sign.expiry < now {
		return Err(anyhow::anyhow!("License key expired"));
	}
	let public_key_hex = product_sign.clone().public_key.clone();
	// remove 0x prefix
	let public_key_hex = public_key_hex.trim_start_matches("0x");
	// hex to U8
	let public_key_bytes = hex::decode(public_key_hex)?;
	let pkey_32: [u8; 32] = public_key_bytes.as_slice().try_into()?;
	let public_key_querent = sr25519::Public::from_raw(pkey_32);
	let signature = product_sign.signature.clone();
	let to_sign = format!(
		"{}{}{}{}",
		product_sign.payload.name,
		product_sign.payload.email,
		product_sign.payload.website,
		product_sign.payload.product.to_string()
	);
	let to_sign = to_sign.as_bytes();
	let signed_payload = wrap_binary_data(to_sign.to_vec());
	// Verify signature
	let verified = signature.verify(&signed_payload[..], &public_key_querent.into());
	if !verified {
		return Err(anyhow::anyhow!("Invalid signature"));
	}
	Ok(true)
}

// Return a ProductRegistrationInfo from a license key
#[allow(deprecated)]
pub fn get_product_info(licence_key: String) -> Result<ProductRegistrationInfo, anyhow::Error> {
	// license_key is a base64 encoded string
	let key = base64::decode(licence_key)?;
	// parse key into ProductRegistrationInfo
	let product_sign: SignedPayload = serde_json::from_slice(&key)?;
	Ok(product_sign.payload)
}

pub fn get_pipeline_count_by_product(licence_key: String) -> Result<usize, anyhow::Error> {
	let info = get_product_info(licence_key)?;
	match info.product {
		ProductType::Rian => Ok(1),
		ProductType::RianPro => Ok(usize::MAX),
		ProductType::RianEnterprise => Ok(usize::MAX),
	}
}

pub fn get_total_sources_by_product(licence_key: String) -> Result<usize, anyhow::Error> {
	let info = get_product_info(licence_key)?;
	match info.product {
		ProductType::Rian => Ok(5),
		ProductType::RianPro => Ok(usize::MAX),
		ProductType::RianEnterprise => Ok(usize::MAX),
	}
}

pub fn is_data_source_allowed_by_product(
	licence_key: String,
	data_source: &CollectorConfig,
) -> Result<bool, anyhow::Error> {
	let info = get_product_info(licence_key);
	if info.is_err() {
		return Ok(false);
	}
	match info.unwrap().product {
		ProductType::Rian => match &data_source.backend {
			Some(proto::semantics::Backend::Files(_)) => Ok(true),
			Some(proto::semantics::Backend::Gcs(_)) => Ok(true),
			_ => Ok(false),
		},
		ProductType::RianPro => Ok(true),
		ProductType::RianEnterprise => Ok(true),
	}
}

pub fn is_discovery_agent_type_allowed(
	licence_key: String,
	agent_type: &DiscoveryAgentType,
) -> Result<bool, anyhow::Error> {
	let info = get_product_info(licence_key)?;
	match info.product {
		ProductType::Rian => match agent_type {
			DiscoveryAgentType::Retriever => Ok(true),
			_ => Ok(false),
		},
		ProductType::RianPro => Ok(true),
		ProductType::RianEnterprise => Ok(true),
	}
}

pub fn is_insight_allowed_by_product(
	licence_key: String,
	insight_id: String,
) -> Result<bool, anyhow::Error> {
	let info = get_product_info(licence_key)?;
	match info.product {
		ProductType::Rian => match insight_id.as_str() {
			"querent.insights.x_ai.openai" => Ok(true),

			_ => Ok(false),
		},
		ProductType::RianPro => Ok(true),
		ProductType::RianEnterprise => Ok(true),
	}
}

const PREFIX: &'static str = "<Bytes>";
const POSTFIX: &'static str = "</Bytes>";

/// Wraps `PREFIX` and `POSTFIX` around a `Vec<u8>`
/// Returns `PREFIX` ++ `data` ++ `POSTFIX`
pub fn wrap_binary_data(data: Vec<u8>) -> Vec<u8> {
	let mut encapsuled = PREFIX.as_bytes().to_vec();
	encapsuled.append(&mut data.clone());
	encapsuled.append(&mut POSTFIX.as_bytes().to_vec());
	encapsuled
}

#[cfg(test)]
mod tests {
	use super::*;
	const TEST_KEY: &str = "eyJzaWduYXR1cmUiOnsiU3IyNTUxOSI6IjB4NjIwNTE4YjcyMWFlOTViZDUzNzMzOWIyNjM4YWQ4MjFiZjgyZmRlMTU5Y2I1ZGFiNGIyMWUyYjk1ZWIzZDUyZDYwOGNlZWMwNDhhNDQxMTAyYmUxYTU5ZTk4YjRhZTJkZDYzN2FiN2U0MjZkZDE0N2UzOTQ4YzNmZTA4MTU5OGUifSwicHVibGljS2V5IjoiMHhhY2I2YjMzMjQ4ZmNmYzUyZmJlMjQwMmE4YzY3ZjRiMGJlZDkzOWExNmQzNDIzZmNmZTlmMjNiYzhiNzI0MjIwIiwicGF5bG9hZCI6eyJuYW1lIjoiSGVsbG8gV29ybGQiLCJlbWFpbCI6ImFlZGFlIiwid2Vic2l0ZSI6ImRkIiwicHJvZHVjdCI6InJpYW4ifSwiZXhwaXJ5IjoxNzIzOTIxMDc1MTE2fQ==";
	#[test]
	fn test_wrap_binary_data() {
		let data = vec![1, 2, 3, 4, 5];
		let wrapped = wrap_binary_data(data.clone());
		let expected = "<Bytes>".as_bytes().to_vec();
		let mut expected = expected.clone();
		expected.append(&mut data.clone());
		expected.append(&mut "</Bytes>".as_bytes().to_vec());
		assert_eq!(wrapped, expected);
	}

	#[test]
	fn test_verify_key() {
		let result = verify_key(TEST_KEY.to_string());
		assert_eq!(result.is_ok(), true);
	}

	#[test]
	fn test_get_product_info() {
		let result = get_product_info(TEST_KEY.to_string());
		assert_eq!(result.is_ok(), true);
	}

	#[test]
	fn test_get_pipeline_count_by_product() {
		let result = get_pipeline_count_by_product(TEST_KEY.to_string());
		assert_eq!(result.is_ok(), true);
	}

	#[test]
	fn test_is_data_source_allowed_by_product() {
		let collector = CollectorConfig {
			name: "test".to_string(),
			backend: Some(proto::semantics::Backend::Files(
				proto::semantics::FileCollectorConfig {
					root_path: "test".to_string(),
					id: "test".to_string(),
				},
			)),
		};
		let result = is_data_source_allowed_by_product(TEST_KEY.to_string(), &collector);
		assert_eq!(result.is_ok(), true);
	}

	#[test]
	fn test_is_discovery_agent_type_allowed() {
		let result =
			is_discovery_agent_type_allowed(TEST_KEY.to_string(), &DiscoveryAgentType::Retriever);
		assert_eq!(result.is_ok(), true);
	}

	#[test]
	fn test_is_insight_allowed_by_product() {
		let result = is_insight_allowed_by_product(
			TEST_KEY.to_string(),
			"querent.insights.x_ai.openai".to_string(),
		);
		assert_eq!(result.is_ok(), true);
	}
}
