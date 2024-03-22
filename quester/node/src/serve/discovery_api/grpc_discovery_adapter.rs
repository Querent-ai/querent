use std::sync::Arc;

use async_trait::async_trait;
use discovery::DiscoveryService;
use proto::{discovery::discovery_server as grpc, error::convert_to_grpc_result};
use tracing::instrument;

#[derive(Clone)]
pub struct DiscoveryAdapter(Arc<dyn DiscoveryService>);

impl From<Arc<dyn DiscoveryService>> for DiscoveryAdapter {
	fn from(service: Arc<dyn DiscoveryService>) -> Self {
		DiscoveryAdapter(service)
	}
}

#[async_trait]
impl grpc::Discovery for DiscoveryAdapter {
	#[instrument(skip(self, request))]
	async fn discover_insights(
		&self,
		request: tonic::Request<proto::discovery::DiscoveryRequest>,
	) -> Result<tonic::Response<proto::discovery::DiscoveryResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::DiscoveryResponse, discovery::error::DiscoveryError> =
			self.0.discover_insights(req).await;
		convert_to_grpc_result(res)
	}
}
