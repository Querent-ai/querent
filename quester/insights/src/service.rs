use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use proto::{DiscoveryRequest, DiscoveryResponse};
use querent_synapse::callbacks::EventType;
use storage::Storage;

#[async_trait]
pub trait DiscoveryService: 'static + Send + Sync {
	/// Discover insights
	async fn discover_insights(
		&self,
		request: DiscoveryRequest,
	) -> crate::Result<DiscoveryResponse>;
}

#[derive(Clone)]
pub struct DiscoveryImpl {
	pub event_storages: HashMap<EventType, Arc<dyn Storage>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
}

impl DiscoveryImpl {
	pub fn new(
		event_storages: HashMap<EventType, Arc<dyn Storage>>,
		index_storages: Vec<Arc<dyn Storage>>,
	) -> Self {
		Self { event_storages, index_storages }
	}
}

#[async_trait]
impl DiscoveryService for DiscoveryImpl {
	async fn discover_insights(
		&self,
		_request: DiscoveryRequest,
	) -> crate::Result<DiscoveryResponse> {
		// TODO: Implement this method utilizing the event_storages and index_storages
		// and return the appropriate response via GraphRag mechanism
		// GraphRag essentially is a graph-based recommendation system that can be used to
		// recommend insights based on the data in the storages
		Ok(DiscoveryResponse::default())
	}
}
