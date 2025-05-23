/// Start a layer agent to query insights from data
/// The agent will respond with insights layereded based on the user's query
/// The agent use vector and graph embeddings to layered insights from data
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LayerSessionRequest {
    /// Name of the layer agent
    #[prost(string, tag = "1")]
    pub agent_name: ::prost::alloc::string::String,
    /// Semantic pipeline ID
    #[prost(string, tag = "2")]
    pub semantic_pipeline_id: ::prost::alloc::string::String,
    /// Layer Session Type
    #[prost(message, optional, tag = "3")]
    pub session_type: ::core::option::Option<LayerAgentType>,
}
/// Session AgentID as a response
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LayerSessionResponse {
    /// The ID of the layer session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
/// Request message for querying insights from data
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[derive(Eq, Hash)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LayerRequest {
    /// The ID of the layer session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    /// The query or question posed by the user
    #[prost(string, tag = "2")]
    pub query: ::prost::alloc::string::String,
    /// The subject - object pairs based on the user selected filter
    #[prost(string, repeated, tag = "3")]
    pub top_pairs: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// Response message containing insights layereded from the data
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LayerResponse {
    /// The ID of the layer session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    /// Query or question posed by the user
    #[prost(string, tag = "2")]
    pub query: ::prost::alloc::string::String,
    /// The insights layereded based on the user's query
    #[prost(message, repeated, tag = "3")]
    pub insights: ::prost::alloc::vec::Vec<Insight>,
    /// The search result page number
    #[prost(int32, tag = "4")]
    pub page_ranking: i32,
}
/// Request to stop the layer session
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopLayerSessionRequest {
    /// The ID of the layer session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
/// Response to stop the layer session
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopLayerSessionResponse {
    /// The ID of the layer session
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
/// Represents an insight layereded from the data
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Insight {
    /// The document id of the search result
    #[prost(string, tag = "1")]
    pub document: ::prost::alloc::string::String,
    /// The document source
    #[prost(string, tag = "2")]
    pub source: ::prost::alloc::string::String,
    /// The attention score retrieved from the search result
    #[prost(string, tag = "3")]
    pub relationship_strength: ::prost::alloc::string::String,
    /// The sentence retrieved from the search resulty
    #[prost(string, tag = "4")]
    pub sentence: ::prost::alloc::string::String,
    /// The tags of the search result, comma separated subject, object , predicate
    #[prost(string, tag = "5")]
    pub tags: ::prost::alloc::string::String,
    /// The top 10 subject_object_pairs, comma separated e.g. subject 1 - object 1, subject 2 - object 2 etc.
    #[prost(string, repeated, tag = "6")]
    pub top_pairs: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LayerSessionRequestInfo {
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub request: ::core::option::Option<LayerSessionRequest>,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LayerSessionRequestInfoList {
    #[prost(message, repeated, tag = "1")]
    pub requests: ::prost::alloc::vec::Vec<LayerSessionRequestInfo>,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Empty {}
/// Generated client implementations.
pub mod layer_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// The Layer service provides a method to query insights from data
    #[derive(Debug, Clone)]
    pub struct LayerClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl LayerClient<tonic::transport::Channel> {
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
    impl<T> LayerClient<T>
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
        ) -> LayerClient<InterceptedService<T, F>>
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
            LayerClient::new(InterceptedService::new(inner, interceptor))
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
        /// Start a layer session to query insights from data
        pub async fn start_layer_session(
            &mut self,
            request: impl tonic::IntoRequest<super::LayerSessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::LayerSessionResponse>,
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
                "/querent.layer.Layer/StartLayerSession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("querent.layer.Layer", "StartLayerSession"));
            self.inner.unary(req, path, codec).await
        }
        /// Query insights from data
        pub async fn layer_insights(
            &mut self,
            request: impl tonic::IntoRequest<super::LayerRequest>,
        ) -> std::result::Result<tonic::Response<super::LayerResponse>, tonic::Status> {
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
                "/querent.layer.Layer/LayerInsights",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("querent.layer.Layer", "LayerInsights"));
            self.inner.unary(req, path, codec).await
        }
        /// Stop the layer session
        pub async fn stop_layer_session(
            &mut self,
            request: impl tonic::IntoRequest<super::StopLayerSessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::StopLayerSessionResponse>,
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
                "/querent.layer.Layer/StopLayerSession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("querent.layer.Layer", "StopLayerSession"));
            self.inner.unary(req, path, codec).await
        }
        /// List all layer sessions
        pub async fn list_layer_sessions(
            &mut self,
            request: impl tonic::IntoRequest<super::Empty>,
        ) -> std::result::Result<
            tonic::Response<super::LayerSessionRequestInfoList>,
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
                "/querent.layer.Layer/ListLayerSessions",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("querent.layer.Layer", "ListLayerSessions"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod layer_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with LayerServer.
    #[async_trait]
    pub trait Layer: Send + Sync + 'static {
        /// Start a layer session to query insights from data
        async fn start_layer_session(
            &self,
            request: tonic::Request<super::LayerSessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::LayerSessionResponse>,
            tonic::Status,
        >;
        /// Query insights from data
        async fn layer_insights(
            &self,
            request: tonic::Request<super::LayerRequest>,
        ) -> std::result::Result<tonic::Response<super::LayerResponse>, tonic::Status>;
        /// Stop the layer session
        async fn stop_layer_session(
            &self,
            request: tonic::Request<super::StopLayerSessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::StopLayerSessionResponse>,
            tonic::Status,
        >;
        /// List all layer sessions
        async fn list_layer_sessions(
            &self,
            request: tonic::Request<super::Empty>,
        ) -> std::result::Result<
            tonic::Response<super::LayerSessionRequestInfoList>,
            tonic::Status,
        >;
    }
    /// The Layer service provides a method to query insights from data
    #[derive(Debug)]
    pub struct LayerServer<T: Layer> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Layer> LayerServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for LayerServer<T>
    where
        T: Layer,
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
                "/querent.layer.Layer/StartLayerSession" => {
                    #[allow(non_camel_case_types)]
                    struct StartLayerSessionSvc<T: Layer>(pub Arc<T>);
                    impl<
                        T: Layer,
                    > tonic::server::UnaryService<super::LayerSessionRequest>
                    for StartLayerSessionSvc<T> {
                        type Response = super::LayerSessionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::LayerSessionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).start_layer_session(request).await
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
                        let method = StartLayerSessionSvc(inner);
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
                "/querent.layer.Layer/LayerInsights" => {
                    #[allow(non_camel_case_types)]
                    struct LayerInsightsSvc<T: Layer>(pub Arc<T>);
                    impl<T: Layer> tonic::server::UnaryService<super::LayerRequest>
                    for LayerInsightsSvc<T> {
                        type Response = super::LayerResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::LayerRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).layer_insights(request).await
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
                        let method = LayerInsightsSvc(inner);
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
                "/querent.layer.Layer/StopLayerSession" => {
                    #[allow(non_camel_case_types)]
                    struct StopLayerSessionSvc<T: Layer>(pub Arc<T>);
                    impl<
                        T: Layer,
                    > tonic::server::UnaryService<super::StopLayerSessionRequest>
                    for StopLayerSessionSvc<T> {
                        type Response = super::StopLayerSessionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::StopLayerSessionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).stop_layer_session(request).await
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
                        let method = StopLayerSessionSvc(inner);
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
                "/querent.layer.Layer/ListLayerSessions" => {
                    #[allow(non_camel_case_types)]
                    struct ListLayerSessionsSvc<T: Layer>(pub Arc<T>);
                    impl<T: Layer> tonic::server::UnaryService<super::Empty>
                    for ListLayerSessionsSvc<T> {
                        type Response = super::LayerSessionRequestInfoList;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::Empty>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).list_layer_sessions(request).await
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
                        let method = ListLayerSessionsSvc(inner);
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
    impl<T: Layer> Clone for LayerServer<T> {
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
    impl<T: Layer> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Layer> tonic::server::NamedService for LayerServer<T> {
        const NAME: &'static str = "querent.layer.Layer";
    }
}
