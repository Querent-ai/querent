use std::{net::SocketAddr, sync::Arc};

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
pub struct SearchServiceClient {
	client_impl: DiscoveryServiceClientImpl,
	grpc_addr: SocketAddr,
}
