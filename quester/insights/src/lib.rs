use client::DiscoveryServiceClient;
use common::Pool;
use error::DiscoveryError;
use querent_synapse::callbacks::EventType;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use storage::Storage;

mod client;
pub mod error;
pub mod service;
pub use service::DiscoveryService;

pub type InsightsPool = Pool<SocketAddr, DiscoveryServiceClient>;

pub type Result<T> = std::result::Result<T, DiscoveryError>;

pub async fn start_discovery_service(
	event_storages: HashMap<EventType, Arc<dyn Storage>>,
	index_storages: Vec<Arc<dyn Storage>>,
) -> anyhow::Result<Arc<dyn DiscoveryService>> {
	let discovery_service = Arc::new(service::DiscoveryImpl::new(event_storages, index_storages));
	Ok(discovery_service)
}
