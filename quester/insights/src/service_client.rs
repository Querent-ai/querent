use tonic::codegen::InterceptedService;
use tower::timeout::Timeout;
use tonic::transport::Channel;


#[derive(Clone)]
enum SearchServiceClientImpl {
    Grpc(
        proto::discovery::discovery_grpc_client::DiscoveryGrpcClient<
            InterceptedService<Timeout<Channel>, SpanContextInterceptor>,
        >,
    ),
}