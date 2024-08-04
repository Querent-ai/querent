#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InsightAnalystRequest {
    /// Registered ID of the insight; call /insights endpoint for available insights
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    /// Optional discovery session ID
    #[prost(string, optional, tag = "2")]
    pub discovery_session_id: ::core::option::Option<::prost::alloc::string::String>,
    /// Optional semantic pipeline ID
    #[prost(string, optional, tag = "3")]
    pub semantic_pipeline_id: ::core::option::Option<::prost::alloc::string::String>,
    /// Additional insight-specific parameters corresponding to the ID
    #[prost(map = "string, string", tag = "4")]
    pub additional_options: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InsightAnalystResponse {
    /// The ID of the insight session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InsightQuery {
    /// The ID of the insight session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    /// The query to be used for the insight
    #[prost(string, tag = "2")]
    pub query: ::prost::alloc::string::String,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InsightQueryResponse {
    /// The ID of the insight session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    /// Hash of the query
    #[prost(string, tag = "2")]
    pub query_hash: ::prost::alloc::string::String,
    /// The response from the insight
    #[prost(string, tag = "3")]
    pub response: ::prost::alloc::string::String,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopInsightSessionRequest {
    /// The ID of the insight session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopInsightSessionResponse {
    /// The ID of the insight session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EmptyInput {}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InsightRequestInfo {
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub request: ::core::option::Option<InsightAnalystRequest>,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InsightRequestInfoList {
    #[prost(message, repeated, tag = "1")]
    pub requests: ::prost::alloc::vec::Vec<InsightRequestInfo>,
}
/// Generated client implementations.
pub mod insight_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// InsightService is a service that provides insights from data
    #[derive(Debug, Clone)]
    pub struct InsightServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl InsightServiceClient<tonic::transport::Channel> {
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
    impl<T> InsightServiceClient<T>
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
        ) -> InsightServiceClient<InterceptedService<T, F>>
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
            InsightServiceClient::new(InterceptedService::new(inner, interceptor))
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
        /// Request creation of an insight session
        pub async fn create_insight_session(
            &mut self,
            request: impl tonic::IntoRequest<super::InsightAnalystRequest>,
        ) -> std::result::Result<
            tonic::Response<super::InsightAnalystResponse>,
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
                "/querent.insights.InsightService/CreateInsightSession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "querent.insights.InsightService",
                        "CreateInsightSession",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Provide the input for an insight and get a response (for query-based insights)
        pub async fn provide_insight_input(
            &mut self,
            request: impl tonic::IntoRequest<super::InsightQuery>,
        ) -> std::result::Result<
            tonic::Response<super::InsightQueryResponse>,
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
                "/querent.insights.InsightService/ProvideInsightInput",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "querent.insights.InsightService",
                        "ProvideInsightInput",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Stop an insight session
        pub async fn stop_insight_session(
            &mut self,
            request: impl tonic::IntoRequest<super::StopInsightSessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::StopInsightSessionResponse>,
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
                "/querent.insights.InsightService/StopInsightSession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "querent.insights.InsightService",
                        "StopInsightSession",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// List all insights requests stored
        pub async fn list_insight_requests(
            &mut self,
            request: impl tonic::IntoRequest<super::EmptyInput>,
        ) -> std::result::Result<
            tonic::Response<super::InsightRequestInfoList>,
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
                "/querent.insights.InsightService/ListInsightRequests",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "querent.insights.InsightService",
                        "ListInsightRequests",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod insight_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with InsightServiceServer.
    #[async_trait]
    pub trait InsightService: Send + Sync + 'static {
        /// Request creation of an insight session
        async fn create_insight_session(
            &self,
            request: tonic::Request<super::InsightAnalystRequest>,
        ) -> std::result::Result<
            tonic::Response<super::InsightAnalystResponse>,
            tonic::Status,
        >;
        /// Provide the input for an insight and get a response (for query-based insights)
        async fn provide_insight_input(
            &self,
            request: tonic::Request<super::InsightQuery>,
        ) -> std::result::Result<
            tonic::Response<super::InsightQueryResponse>,
            tonic::Status,
        >;
        /// Stop an insight session
        async fn stop_insight_session(
            &self,
            request: tonic::Request<super::StopInsightSessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::StopInsightSessionResponse>,
            tonic::Status,
        >;
        /// List all insights requests stored
        async fn list_insight_requests(
            &self,
            request: tonic::Request<super::EmptyInput>,
        ) -> std::result::Result<
            tonic::Response<super::InsightRequestInfoList>,
            tonic::Status,
        >;
    }
    /// InsightService is a service that provides insights from data
    #[derive(Debug)]
    pub struct InsightServiceServer<T: InsightService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: InsightService> InsightServiceServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for InsightServiceServer<T>
    where
        T: InsightService,
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
                "/querent.insights.InsightService/CreateInsightSession" => {
                    #[allow(non_camel_case_types)]
                    struct CreateInsightSessionSvc<T: InsightService>(pub Arc<T>);
                    impl<
                        T: InsightService,
                    > tonic::server::UnaryService<super::InsightAnalystRequest>
                    for CreateInsightSessionSvc<T> {
                        type Response = super::InsightAnalystResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::InsightAnalystRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).create_insight_session(request).await
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
                        let method = CreateInsightSessionSvc(inner);
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
                "/querent.insights.InsightService/ProvideInsightInput" => {
                    #[allow(non_camel_case_types)]
                    struct ProvideInsightInputSvc<T: InsightService>(pub Arc<T>);
                    impl<
                        T: InsightService,
                    > tonic::server::UnaryService<super::InsightQuery>
                    for ProvideInsightInputSvc<T> {
                        type Response = super::InsightQueryResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::InsightQuery>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).provide_insight_input(request).await
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
                        let method = ProvideInsightInputSvc(inner);
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
                "/querent.insights.InsightService/StopInsightSession" => {
                    #[allow(non_camel_case_types)]
                    struct StopInsightSessionSvc<T: InsightService>(pub Arc<T>);
                    impl<
                        T: InsightService,
                    > tonic::server::UnaryService<super::StopInsightSessionRequest>
                    for StopInsightSessionSvc<T> {
                        type Response = super::StopInsightSessionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::StopInsightSessionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).stop_insight_session(request).await
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
                        let method = StopInsightSessionSvc(inner);
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
                "/querent.insights.InsightService/ListInsightRequests" => {
                    #[allow(non_camel_case_types)]
                    struct ListInsightRequestsSvc<T: InsightService>(pub Arc<T>);
                    impl<
                        T: InsightService,
                    > tonic::server::UnaryService<super::EmptyInput>
                    for ListInsightRequestsSvc<T> {
                        type Response = super::InsightRequestInfoList;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::EmptyInput>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).list_insight_requests(request).await
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
                        let method = ListInsightRequestsSvc(inner);
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
    impl<T: InsightService> Clone for InsightServiceServer<T> {
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
    impl<T: InsightService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: InsightService> tonic::server::NamedService for InsightServiceServer<T> {
        const NAME: &'static str = "querent.insights.InsightService";
    }
}
