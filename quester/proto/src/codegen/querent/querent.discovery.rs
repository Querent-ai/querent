/// Start a discovery agent to query insights from data
/// The agent will respond with insights discovered based on the user's query
/// The agent use vector and graph embeddings to discover insights from data
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DiscoverySessionRequest {
    /// Name of the discovery agent
    #[prost(string, tag = "1")]
    pub agent_name: ::prost::alloc::string::String,
    /// Optional storage storage_configs
    #[prost(message, repeated, tag = "5")]
    pub storage_configs: ::prost::alloc::vec::Vec<StorageConfig>,
    /// Max message memory size
    #[prost(int32, tag = "6")]
    pub max_message_memory_size: i32,
    /// Max query tokens size
    #[prost(int32, tag = "7")]
    pub max_query_tokens_size: i32,
    /// Semantic pipeline ID
    #[prost(string, tag = "8")]
    pub semantic_pipeline_id: ::prost::alloc::string::String,
    /// OpenAI API key
    #[prost(string, tag = "9")]
    pub openai_api_key: ::prost::alloc::string::String,
    /// Discovery Session Type
    #[prost(message, optional, tag = "10")]
    pub session_type: ::core::option::Option<DiscoveryAgentType>,
}
/// Session AgentID as a response
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DiscoverySessionResponse {
    /// The ID of the discovery session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
/// Request message for querying insights from data
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[derive(Eq, Hash)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DiscoveryRequest {
    /// The ID of the discovery session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    /// The query or question posed by the user
    #[prost(string, tag = "2")]
    pub query: ::prost::alloc::string::String,
}
/// Response message containing insights discovered from the data
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DiscoveryResponse {
    /// The ID of the discovery session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    /// The insights discovered based on the user's query
    #[prost(message, repeated, tag = "2")]
    pub insights: ::prost::alloc::vec::Vec<Insight>,
}
/// Request to stop the discovery session
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopDiscoverySessionRequest {
    /// The ID of the discovery session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
/// Response to stop the discovery session
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopDiscoverySessionResponse {
    /// The ID of the discovery session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
/// Represents an insight discovered from the data
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
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
/// Generated client implementations.
pub mod discovery_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// The Discovery service provides a method to query insights from data
    #[derive(Debug, Clone)]
    pub struct DiscoveryClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl DiscoveryClient<tonic::transport::Channel> {
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
    impl<T> DiscoveryClient<T>
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
        ) -> DiscoveryClient<InterceptedService<T, F>>
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
            DiscoveryClient::new(InterceptedService::new(inner, interceptor))
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
        /// Start a discovery session to query insights from data
        pub async fn start_discovery_session(
            &mut self,
            request: impl tonic::IntoRequest<super::DiscoverySessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::DiscoverySessionResponse>,
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
                "/querent.discovery.Discovery/StartDiscoverySession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "querent.discovery.Discovery",
                        "StartDiscoverySession",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Query insights from data
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
        /// Stop the discovery session
        pub async fn stop_discovery_session(
            &mut self,
            request: impl tonic::IntoRequest<super::StopDiscoverySessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::StopDiscoverySessionResponse>,
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
                "/querent.discovery.Discovery/StopDiscoverySession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "querent.discovery.Discovery",
                        "StopDiscoverySession",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod discovery_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with DiscoveryServer.
    #[async_trait]
    pub trait Discovery: Send + Sync + 'static {
        /// Start a discovery session to query insights from data
        async fn start_discovery_session(
            &self,
            request: tonic::Request<super::DiscoverySessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::DiscoverySessionResponse>,
            tonic::Status,
        >;
        /// Query insights from data
        async fn discover_insights(
            &self,
            request: tonic::Request<super::DiscoveryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::DiscoveryResponse>,
            tonic::Status,
        >;
        /// Stop the discovery session
        async fn stop_discovery_session(
            &self,
            request: tonic::Request<super::StopDiscoverySessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::StopDiscoverySessionResponse>,
            tonic::Status,
        >;
    }
    /// The Discovery service provides a method to query insights from data
    #[derive(Debug)]
    pub struct DiscoveryServer<T: Discovery> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Discovery> DiscoveryServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for DiscoveryServer<T>
    where
        T: Discovery,
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
                "/querent.discovery.Discovery/StartDiscoverySession" => {
                    #[allow(non_camel_case_types)]
                    struct StartDiscoverySessionSvc<T: Discovery>(pub Arc<T>);
                    impl<
                        T: Discovery,
                    > tonic::server::UnaryService<super::DiscoverySessionRequest>
                    for StartDiscoverySessionSvc<T> {
                        type Response = super::DiscoverySessionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DiscoverySessionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).start_discovery_session(request).await
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
                        let method = StartDiscoverySessionSvc(inner);
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
                "/querent.discovery.Discovery/DiscoverInsights" => {
                    #[allow(non_camel_case_types)]
                    struct DiscoverInsightsSvc<T: Discovery>(pub Arc<T>);
                    impl<
                        T: Discovery,
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
                "/querent.discovery.Discovery/StopDiscoverySession" => {
                    #[allow(non_camel_case_types)]
                    struct StopDiscoverySessionSvc<T: Discovery>(pub Arc<T>);
                    impl<
                        T: Discovery,
                    > tonic::server::UnaryService<super::StopDiscoverySessionRequest>
                    for StopDiscoverySessionSvc<T> {
                        type Response = super::StopDiscoverySessionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::StopDiscoverySessionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).stop_discovery_session(request).await
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
                        let method = StopDiscoverySessionSvc(inner);
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
    impl<T: Discovery> Clone for DiscoveryServer<T> {
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
    impl<T: Discovery> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Discovery> tonic::server::NamedService for DiscoveryServer<T> {
        const NAME: &'static str = "querent.discovery.Discovery";
    }
}
