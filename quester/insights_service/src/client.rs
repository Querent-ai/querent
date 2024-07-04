use std::{fmt, net::SocketAddr, sync::Arc};

use crate::service::InsightService;
use proto::SpanContextInterceptor;
use tonic::{codegen::InterceptedService, transport::Channel};
use tower::timeout::Timeout;

#[derive(Clone)]
enum InsightServiceClientImpl {
	Rest(Arc<dyn InsightService>),
	Grpc(
		proto::insights::insight_service_client::InsightServiceClient<
			InterceptedService<Timeout<Channel>, SpanContextInterceptor>,
		>,
	),
}

#[derive(Clone)]
pub struct InsightServiceClient {
	client_impl: InsightServiceClientImpl,
	grpc_addr: SocketAddr,
}

impl fmt::Debug for InsightServiceClient {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		match &self.client_impl {
			InsightServiceClientImpl::Rest(_service) => {
				write!(formatter, "Rest({:?})", self.grpc_addr)
			},
			InsightServiceClientImpl::Grpc(_grpc_client) => {
				write!(formatter, "Grpc({:?})", self.grpc_addr)
			},
		}
	}
}

impl InsightServiceClient {
	/// Create a discovery service client instance given a gRPC client and gRPC address.
	pub fn from_grpc_client(
		client: proto::insights::insight_service_client::InsightServiceClient<
			InterceptedService<Timeout<Channel>, SpanContextInterceptor>,
		>,
		grpc_addr: SocketAddr,
	) -> Self {
		InsightServiceClient { client_impl: InsightServiceClientImpl::Grpc(client), grpc_addr }
	}

	/// Create a search service client instance given a search service and gRPC address.
	pub fn from_service(service: Arc<dyn InsightService>, grpc_addr: SocketAddr) -> Self {
		InsightServiceClient { client_impl: InsightServiceClientImpl::Rest(service), grpc_addr }
	}

	/// Return the grpc_addr the underlying client connects to.
	pub fn grpc_addr(&self) -> SocketAddr {
		self.grpc_addr
	}

	/// Returns whether the underlying client is rest or remote.
	#[cfg(any(test, feature = "testsuite"))]
	pub fn is_local(&self) -> bool {
		matches!(self.client_impl, InsightServiceClientImpl::Rest(_))
	}
}
