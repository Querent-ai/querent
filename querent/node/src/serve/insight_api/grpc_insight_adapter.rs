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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use async_trait::async_trait;
use insights::InsightError;
use proto::{error::convert_to_grpc_result, insights::insight_service_server as grpc};
use std::sync::Arc;
use tracing::instrument;

use crate::insight_api::insights_service::InsightService;

#[derive(Clone)]
pub struct InsightAdapter(Arc<dyn InsightService>);

impl From<Arc<dyn InsightService>> for InsightAdapter {
	fn from(service: Arc<dyn InsightService>) -> Self {
		InsightAdapter(service)
	}
}

#[async_trait]
impl grpc::InsightService for InsightAdapter {
	#[instrument(skip(self, request))]
	async fn create_insight_session(
		&self,
		request: tonic::Request<proto::insights::InsightAnalystRequest>,
	) -> Result<tonic::Response<proto::insights::InsightAnalystResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::InsightAnalystResponse, InsightError> =
			self.0.create_insight_session(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, request))]
	async fn provide_insight_input(
		&self,
		request: tonic::Request<proto::insights::InsightQuery>,
	) -> Result<tonic::Response<proto::insights::InsightQueryResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::InsightQueryResponse, InsightError> =
			self.0.provide_insight_input(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, request))]
	async fn stop_insight_session(
		&self,
		request: tonic::Request<proto::insights::StopInsightSessionRequest>,
	) -> Result<tonic::Response<proto::insights::StopInsightSessionResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::StopInsightSessionResponse, InsightError> =
			self.0.stop_insight_session(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, request))]
	async fn list_insight_requests(
		&self,
		request: tonic::Request<proto::insights::EmptyInput>,
	) -> Result<tonic::Response<proto::insights::InsightRequestInfoList>, tonic::Status> {
		let _req = request.into_inner();
		let res: Result<proto::InsightRequestInfoList, InsightError> =
			self.0.get_insight_request_list().await;
		convert_to_grpc_result(res)
	}
}
