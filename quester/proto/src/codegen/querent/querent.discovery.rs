/// Request message for querying insights from data
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DiscoveryRequest {
    /// The query or question posed by the user
    #[prost(string, tag = "1")]
    pub query: ::prost::alloc::string::String,
}
/// Response message containing insights discovered from the data
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DiscoveryResponse {
    /// The insights discovered based on the user's query
    #[prost(message, repeated, tag = "1")]
    pub insights: ::prost::alloc::vec::Vec<Insight>,
}
/// Represents an insight discovered from the data
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Insight {
    /// The title or summary of the insight
    #[prost(string, tag = "1")]
    pub title: ::prost::alloc::string::String,
    /// The detailed description or explanation of the insight
    #[prost(string, tag = "2")]
    pub description: ::prost::alloc::string::String,
}
/// BEGIN
#[allow(unused_imports)]
use std::str::FromStr;
use tower::{Layer, Service, ServiceExt};
use common::tower::RpcName;
impl RpcName for DiscoveryRequest {
    fn rpc_name() -> &'static str {
        "discover_insights"
    }
}
#[cfg_attr(any(test, feature = "testsuite"), mockall::automock)]
#[async_trait::async_trait]
pub trait Discovery: std::fmt::Debug + dyn_clone::DynClone + Send + Sync + 'static {
    /// Discovers insights from data based on the user's query
    async fn discover_insights(
        &mut self,
        request: DiscoveryRequest,
    ) -> crate::discovery::DiscoveryResult<DiscoveryResponse>;
}
dyn_clone::clone_trait_object!(Discovery);
#[cfg(any(test, feature = "testsuite"))]
impl Clone for MockDiscovery {
    fn clone(&self) -> Self {
        MockDiscovery::new()
    }
}
#[derive(Debug, Clone)]
pub struct DiscoveryClient {
    inner: Box<dyn Discovery>,
}
impl DiscoveryClient {
    pub fn new<T>(instance: T) -> Self
    where
        T: Discovery,
    {
        #[cfg(any(test, feature = "testsuite"))]
        assert!(
            std::any::TypeId::of:: < T > () != std::any::TypeId::of:: < MockDiscovery >
            (),
            "`MockDiscovery` must be wrapped in a `MockDiscoveryWrapper`. Use `MockDiscovery::from(mock)` to instantiate the client."
        );
        Self { inner: Box::new(instance) }
    }
    pub fn as_grpc_service(
        &self,
        max_message_size: bytesize::ByteSize,
    ) -> discovery_grpc_server::DiscoveryGrpcServer<DiscoveryGrpcServerAdapter> {
        let adapter = DiscoveryGrpcServerAdapter::new(self.clone());
        discovery_grpc_server::DiscoveryGrpcServer::new(adapter)
            .max_decoding_message_size(max_message_size.0 as usize)
            .max_encoding_message_size(max_message_size.0 as usize)
    }
    pub fn from_channel(
        addr: std::net::SocketAddr,
        channel: tonic::transport::Channel,
        max_message_size: bytesize::ByteSize,
    ) -> Self {
        let (_, connection_keys_watcher) = tokio::sync::watch::channel(
            std::collections::HashSet::from_iter([addr]),
        );
        let client = discovery_grpc_client::DiscoveryGrpcClient::new(channel)
            .max_decoding_message_size(max_message_size.0 as usize)
            .max_encoding_message_size(max_message_size.0 as usize);
        let adapter = DiscoveryGrpcClientAdapter::new(client, connection_keys_watcher);
        Self::new(adapter)
    }
    pub fn from_balance_channel(
        balance_channel: common::tower::BalanceChannel<std::net::SocketAddr>,
        max_message_size: bytesize::ByteSize,
    ) -> DiscoveryClient {
        let connection_keys_watcher = balance_channel.connection_keys_watcher();
        let client = discovery_grpc_client::DiscoveryGrpcClient::new(balance_channel)
            .max_decoding_message_size(max_message_size.0 as usize)
            .max_encoding_message_size(max_message_size.0 as usize);
        let adapter = DiscoveryGrpcClientAdapter::new(client, connection_keys_watcher);
        Self::new(adapter)
    }
    pub fn from_mailbox<A>(mailbox: actors::MessageBus<A>) -> Self
    where
        A: actors::Actor + std::fmt::Debug + Send + 'static,
        DiscoveryMessageBus<A>: Discovery,
    {
        DiscoveryClient::new(DiscoveryMessageBus::new(mailbox))
    }
    pub fn tower() -> DiscoveryTowerLayerStack {
        DiscoveryTowerLayerStack::default()
    }
    #[cfg(any(test, feature = "testsuite"))]
    pub fn mock() -> MockDiscovery {
        MockDiscovery::new()
    }
}
#[async_trait::async_trait]
impl Discovery for DiscoveryClient {
    async fn discover_insights(
        &mut self,
        request: DiscoveryRequest,
    ) -> crate::discovery::DiscoveryResult<DiscoveryResponse> {
        self.inner.discover_insights(request).await
    }
}
#[cfg(any(test, feature = "testsuite"))]
pub mod discovery_mock {
    use super::*;
    #[derive(Debug, Clone)]
    struct MockDiscoveryWrapper {
        inner: std::sync::Arc<tokio::sync::Mutex<MockDiscovery>>,
    }
    #[async_trait::async_trait]
    impl Discovery for MockDiscoveryWrapper {
        async fn discover_insights(
            &mut self,
            request: super::DiscoveryRequest,
        ) -> crate::discovery::DiscoveryResult<super::DiscoveryResponse> {
            self.inner.lock().await.discover_insights(request).await
        }
    }
    impl From<MockDiscovery> for DiscoveryClient {
        fn from(mock: MockDiscovery) -> Self {
            let mock_wrapper = MockDiscoveryWrapper {
                inner: std::sync::Arc::new(tokio::sync::Mutex::new(mock)),
            };
            DiscoveryClient::new(mock_wrapper)
        }
    }
}
pub type BoxFuture<T, E> = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'static>,
>;
impl tower::Service<DiscoveryRequest> for Box<dyn Discovery> {
    type Response = DiscoveryResponse;
    type Error = crate::discovery::DiscoveryError;
    type Future = BoxFuture<Self::Response, Self::Error>;
    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: DiscoveryRequest) -> Self::Future {
        let mut svc = self.clone();
        let fut = async move { svc.discover_insights(request).await };
        Box::pin(fut)
    }
}
/// A tower service stack is a set of tower services.
#[derive(Debug)]
struct DiscoveryTowerServiceStack {
    inner: Box<dyn Discovery>,
    discover_insights_svc: common::tower::BoxService<
        DiscoveryRequest,
        DiscoveryResponse,
        crate::discovery::DiscoveryError,
    >,
}
impl Clone for DiscoveryTowerServiceStack {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            discover_insights_svc: self.discover_insights_svc.clone(),
        }
    }
}
#[async_trait::async_trait]
impl Discovery for DiscoveryTowerServiceStack {
    async fn discover_insights(
        &mut self,
        request: DiscoveryRequest,
    ) -> crate::discovery::DiscoveryResult<DiscoveryResponse> {
        self.discover_insights_svc.ready().await?.call(request).await
    }
}
type DiscoverInsightsLayer = common::tower::BoxLayer<
    common::tower::BoxService<
        DiscoveryRequest,
        DiscoveryResponse,
        crate::discovery::DiscoveryError,
    >,
    DiscoveryRequest,
    DiscoveryResponse,
    crate::discovery::DiscoveryError,
>;
#[derive(Debug, Default)]
pub struct DiscoveryTowerLayerStack {
    discover_insights_layers: Vec<DiscoverInsightsLayer>,
}
impl DiscoveryTowerLayerStack {
    pub fn stack_layer<L>(mut self, layer: L) -> Self
    where
        L: tower::Layer<
                common::tower::BoxService<
                    DiscoveryRequest,
                    DiscoveryResponse,
                    crate::discovery::DiscoveryError,
                >,
            > + Clone + Send + Sync + 'static,
        <L as tower::Layer<
            common::tower::BoxService<
                DiscoveryRequest,
                DiscoveryResponse,
                crate::discovery::DiscoveryError,
            >,
        >>::Service: tower::Service<
                DiscoveryRequest,
                Response = DiscoveryResponse,
                Error = crate::discovery::DiscoveryError,
            > + Clone + Send + Sync + 'static,
        <<L as tower::Layer<
            common::tower::BoxService<
                DiscoveryRequest,
                DiscoveryResponse,
                crate::discovery::DiscoveryError,
            >,
        >>::Service as tower::Service<DiscoveryRequest>>::Future: Send + 'static,
    {
        self.discover_insights_layers.push(common::tower::BoxLayer::new(layer.clone()));
        self
    }
    pub fn stack_discover_insights_layer<L>(mut self, layer: L) -> Self
    where
        L: tower::Layer<
                common::tower::BoxService<
                    DiscoveryRequest,
                    DiscoveryResponse,
                    crate::discovery::DiscoveryError,
                >,
            > + Send + Sync + 'static,
        L::Service: tower::Service<
                DiscoveryRequest,
                Response = DiscoveryResponse,
                Error = crate::discovery::DiscoveryError,
            > + Clone + Send + Sync + 'static,
        <L::Service as tower::Service<DiscoveryRequest>>::Future: Send + 'static,
    {
        self.discover_insights_layers.push(common::tower::BoxLayer::new(layer));
        self
    }
    pub fn build<T>(self, instance: T) -> DiscoveryClient
    where
        T: Discovery,
    {
        self.build_from_boxed(Box::new(instance))
    }
    pub fn build_from_channel(
        self,
        addr: std::net::SocketAddr,
        channel: tonic::transport::Channel,
        max_message_size: bytesize::ByteSize,
    ) -> DiscoveryClient {
        self.build_from_boxed(
            Box::new(DiscoveryClient::from_channel(addr, channel, max_message_size)),
        )
    }
    pub fn build_from_balance_channel(
        self,
        balance_channel: common::tower::BalanceChannel<std::net::SocketAddr>,
        max_message_size: bytesize::ByteSize,
    ) -> DiscoveryClient {
        self.build_from_boxed(
            Box::new(
                DiscoveryClient::from_balance_channel(balance_channel, max_message_size),
            ),
        )
    }
    pub fn build_from_mailbox<A>(self, mailbox: actors::MessageBus<A>) -> DiscoveryClient
    where
        A: actors::Actor + std::fmt::Debug + Send + 'static,
        DiscoveryMessageBus<A>: Discovery,
    {
        self.build_from_boxed(Box::new(DiscoveryMessageBus::new(mailbox)))
    }
    fn build_from_boxed(self, boxed_instance: Box<dyn Discovery>) -> DiscoveryClient {
        let discover_insights_svc = self
            .discover_insights_layers
            .into_iter()
            .rev()
            .fold(
                common::tower::BoxService::new(boxed_instance.clone()),
                |svc, layer| layer.layer(svc),
            );
        let tower_svc_stack = DiscoveryTowerServiceStack {
            inner: boxed_instance.clone(),
            discover_insights_svc,
        };
        DiscoveryClient::new(tower_svc_stack)
    }
}
#[derive(Debug, Clone)]
struct MailboxAdapter<A: actors::Actor, E> {
    inner: actors::MessageBus<A>,
    phantom: std::marker::PhantomData<E>,
}
impl<A, E> std::ops::Deref for MailboxAdapter<A, E>
where
    A: actors::Actor,
{
    type Target = actors::MessageBus<A>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
#[derive(Debug)]
pub struct DiscoveryMessageBus<A: actors::Actor> {
    inner: MailboxAdapter<A, crate::discovery::DiscoveryError>,
}
impl<A: actors::Actor> DiscoveryMessageBus<A> {
    pub fn new(instance: actors::MessageBus<A>) -> Self {
        let inner = MailboxAdapter {
            inner: instance,
            phantom: std::marker::PhantomData,
        };
        Self { inner }
    }
}
impl<A: actors::Actor> Clone for DiscoveryMessageBus<A> {
    fn clone(&self) -> Self {
        let inner = MailboxAdapter {
            inner: self.inner.clone(),
            phantom: std::marker::PhantomData,
        };
        Self { inner }
    }
}
impl<A, M, T, E> tower::Service<M> for DiscoveryMessageBus<A>
where
    A: actors::Actor + actors::DeferableReplyHandler<M, Reply = Result<T, E>> + Send
        + 'static,
    M: std::fmt::Debug + Send + 'static,
    T: Send + 'static,
    E: std::fmt::Debug + Send + 'static,
    crate::discovery::DiscoveryError: From<actors::AskError<E>>,
{
    type Response = T;
    type Error = crate::discovery::DiscoveryError;
    type Future = BoxFuture<Self::Response, Self::Error>;
    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        //! This does not work with balance middlewares such as `tower::balance::pool::Pool` because
        //! this always returns `Poll::Ready`. The fix is to acquire a permit from the
        //! mailbox in `poll_ready` and consume it in `call`.
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, message: M) -> Self::Future {
        let mailbox = self.inner.clone();
        let fut = async move {
            mailbox.ask_for_res(message).await.map_err(|error| error.into())
        };
        Box::pin(fut)
    }
}
#[async_trait::async_trait]
impl<A> Discovery for DiscoveryMessageBus<A>
where
    A: actors::Actor + std::fmt::Debug,
    DiscoveryMessageBus<
        A,
    >: tower::Service<
        DiscoveryRequest,
        Response = DiscoveryResponse,
        Error = crate::discovery::DiscoveryError,
        Future = BoxFuture<DiscoveryResponse, crate::discovery::DiscoveryError>,
    >,
{
    async fn discover_insights(
        &mut self,
        request: DiscoveryRequest,
    ) -> crate::discovery::DiscoveryResult<DiscoveryResponse> {
        self.call(request).await
    }
}
#[derive(Debug, Clone)]
pub struct DiscoveryGrpcClientAdapter<T> {
    inner: T,
    #[allow(dead_code)]
    connection_addrs_rx: tokio::sync::watch::Receiver<
        std::collections::HashSet<std::net::SocketAddr>,
    >,
}
impl<T> DiscoveryGrpcClientAdapter<T> {
    pub fn new(
        instance: T,
        connection_addrs_rx: tokio::sync::watch::Receiver<
            std::collections::HashSet<std::net::SocketAddr>,
        >,
    ) -> Self {
        Self {
            inner: instance,
            connection_addrs_rx,
        }
    }
}
#[async_trait::async_trait]
impl<T> Discovery
for DiscoveryGrpcClientAdapter<discovery_grpc_client::DiscoveryGrpcClient<T>>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody> + std::fmt::Debug + Clone + Send
        + Sync + 'static,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError>
        + Send,
    T::Future: Send,
{
    async fn discover_insights(
        &mut self,
        request: DiscoveryRequest,
    ) -> crate::discovery::DiscoveryResult<DiscoveryResponse> {
        self.inner
            .discover_insights(request)
            .await
            .map(|response| response.into_inner())
            .map_err(crate::error::grpc_status_to_service_error)
    }
}
#[derive(Debug)]
pub struct DiscoveryGrpcServerAdapter {
    inner: Box<dyn Discovery>,
}
impl DiscoveryGrpcServerAdapter {
    pub fn new<T>(instance: T) -> Self
    where
        T: Discovery,
    {
        Self { inner: Box::new(instance) }
    }
}
#[async_trait::async_trait]
impl discovery_grpc_server::DiscoveryGrpc for DiscoveryGrpcServerAdapter {
    async fn discover_insights(
        &self,
        request: tonic::Request<DiscoveryRequest>,
    ) -> Result<tonic::Response<DiscoveryResponse>, tonic::Status> {
        self.inner
            .clone()
            .discover_insights(request.into_inner())
            .await
            .map(tonic::Response::new)
            .map_err(crate::error::grpc_error_to_grpc_status)
    }
}
/// Generated client implementations.
pub mod discovery_grpc_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// The Discovery service provides a method to query insights from data
    #[derive(Debug, Clone)]
    pub struct DiscoveryGrpcClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl DiscoveryGrpcClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> DiscoveryGrpcClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> DiscoveryGrpcClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            DiscoveryGrpcClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Discovers insights from data based on the user's query
        pub async fn discover_insights(
            &mut self,
            request: impl tonic::IntoRequest<super::DiscoveryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::DiscoveryResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/querent.discovery.Discovery/DiscoverInsights",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("querent.discovery.Discovery", "DiscoverInsights"),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod discovery_grpc_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with DiscoveryGrpcServer.
    #[async_trait]
    pub trait DiscoveryGrpc: Send + Sync + 'static {
        /// Discovers insights from data based on the user's query
        async fn discover_insights(
            &self,
            request: tonic::Request<super::DiscoveryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::DiscoveryResponse>,
            tonic::Status,
        >;
    }
    /// The Discovery service provides a method to query insights from data
    #[derive(Debug)]
    pub struct DiscoveryGrpcServer<T: DiscoveryGrpc> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: DiscoveryGrpc> DiscoveryGrpcServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for DiscoveryGrpcServer<T>
    where
        T: DiscoveryGrpc,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/querent.discovery.Discovery/DiscoverInsights" => {
                    #[allow(non_camel_case_types)]
                    struct DiscoverInsightsSvc<T: DiscoveryGrpc>(pub Arc<T>);
                    impl<
                        T: DiscoveryGrpc,
                    > tonic::server::UnaryService<super::DiscoveryRequest>
                    for DiscoverInsightsSvc<T> {
                        type Response = super::DiscoveryResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DiscoveryRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).discover_insights(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DiscoverInsightsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: DiscoveryGrpc> Clone for DiscoveryGrpcServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: DiscoveryGrpc> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: DiscoveryGrpc> tonic::server::NamedService for DiscoveryGrpcServer<T> {
        const NAME: &'static str = "querent.discovery.Discovery";
    }
}
