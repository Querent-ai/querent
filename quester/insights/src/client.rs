use std::{fmt, net::SocketAddr, sync::Arc};

use crate::service::DiscoveryService;
use proto::SpanContextInterceptor;
use tonic::{codegen::InterceptedService, transport::Channel};
use tower::timeout::Timeout;

#[derive(Clone)]
enum DiscoveryServiceClientImpl {
	Rest(Arc<dyn DiscoveryService>),
	Grpc(
		proto::discovery::discovery_grpc_client::DiscoveryGrpcClient<
			InterceptedService<Timeout<Channel>, SpanContextInterceptor>,
		>,
	),
}

#[derive(Clone)]
pub struct DiscoveryServiceClient {
	client_impl: DiscoveryServiceClientImpl,
	grpc_addr: SocketAddr,
}

impl fmt::Debug for DiscoveryServiceClient {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		match &self.client_impl {
			DiscoveryServiceClientImpl::Rest(_service) => {
				write!(formatter, "Rest({:?})", self.grpc_addr)
			},
			DiscoveryServiceClientImpl::Grpc(_grpc_client) => {
				write!(formatter, "Grpc({:?})", self.grpc_addr)
			},
		}
	}
}

impl DiscoveryServiceClient {
	/// Create a discovery service client instance given a gRPC client and gRPC address.
	pub fn from_grpc_client(
		client: proto::discovery::discovery_grpc_client::DiscoveryGrpcClient<
			InterceptedService<Timeout<Channel>, SpanContextInterceptor>,
		>,
		grpc_addr: SocketAddr,
	) -> Self {
		DiscoveryServiceClient { client_impl: DiscoveryServiceClientImpl::Grpc(client), grpc_addr }
	}

	/// Create a search service client instance given a search service and gRPC address.
	pub fn from_service(service: Arc<dyn DiscoveryService>, grpc_addr: SocketAddr) -> Self {
		DiscoveryServiceClient { client_impl: DiscoveryServiceClientImpl::Rest(service), grpc_addr }
	}

	/// Return the grpc_addr the underlying client connects to.
	pub fn grpc_addr(&self) -> SocketAddr {
		self.grpc_addr
	}

	/// Returns whether the underlying client is rest or remote.
	#[cfg(any(test, feature = "testsuite"))]
	pub fn is_local(&self) -> bool {
		matches!(self.client_impl, DiscoveryServiceClientImpl::Rest(_))
	}
}
