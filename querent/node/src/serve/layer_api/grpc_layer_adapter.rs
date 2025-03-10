// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use std::sync::Arc;

use crate::layer_api::layer_service::{error::LayerError, LayerService};
use async_trait::async_trait;
use proto::{error::convert_to_grpc_result, layer::layer_server as grpc};
use tracing::instrument;

#[derive(Clone)]
pub struct LayerAdapter(Arc<dyn LayerService>);

impl From<Arc<dyn LayerService>> for LayerAdapter {
	fn from(service: Arc<dyn LayerService>) -> Self {
		LayerAdapter(service)
	}
}

#[async_trait]
impl grpc::Layer for LayerAdapter {
	#[instrument(skip(self, request))]
	async fn start_layer_session(
		&self,
		request: tonic::Request<proto::layer::LayerSessionRequest>,
	) -> Result<tonic::Response<proto::layer::LayerSessionResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::layer::LayerSessionResponse, LayerError> =
			self.0.start_layer_session(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, request))]
	async fn layer_insights(
		&self,
		request: tonic::Request<proto::layer::LayerRequest>,
	) -> Result<tonic::Response<proto::layer::LayerResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::layer::LayerResponse, LayerError> = self.0.layer_insights(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, request))]
	async fn stop_layer_session(
		&self,
		request: tonic::Request<proto::layer::StopLayerSessionRequest>,
	) -> Result<tonic::Response<proto::layer::StopLayerSessionResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::layer::StopLayerSessionResponse, LayerError> =
			self.0.stop_layer_session(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, empty))]
	async fn list_layer_sessions(
		&self,
		empty: tonic::Request<proto::layer::Empty>,
	) -> Result<tonic::Response<proto::layer::LayerSessionRequestInfoList>, tonic::Status> {
		let _req = empty.into_inner();
		let res: Result<proto::layer::LayerSessionRequestInfoList, LayerError> =
			self.0.get_layer_session_list().await;
		convert_to_grpc_result(res)
	}
}
