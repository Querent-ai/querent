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

use insights::{
	all_insights_info_available, CustomInsightOption, InsightCustomOptionValue, InsightError,
	InsightErrorKind, InsightInfo,
};
use proto::{
	InsightAnalystRequest, InsightAnalystResponse, InsightQuery, InsightQueryResponse,
	InsightRequestInfo, InsightRequestInfoList, StopInsightSessionRequest,
	StopInsightSessionResponse,
};
use std::sync::Arc;
use warp::{reject::Rejection, Filter};

use crate::{
	extract_format_from_qs, insight_api::insights_service::InsightService, make_json_api_response,
	serve::require,
};

#[derive(utoipa::OpenApi)]
#[openapi(
	paths(
		list_insights,
		start_insight_session_handler,
		insights_prompt_handler,
		stop_insight_session_handler,
		get_pipelines_history
	),
	components(schemas(
		InsightInfo,
		CustomInsightOption,
		InsightCustomOptionValue,
		InsightAnalystRequest,
		InsightAnalystResponse,
		InsightQuery,
		InsightQueryResponse,
		StopInsightSessionRequest,
		StopInsightSessionResponse,
		InsightRequestInfoList,
		InsightRequestInfo
	),)
)]
pub struct InsightsApi;

#[utoipa::path(
    get,
    tag = "Insights",
    path = "/insights/installed",
    responses(
        (status = 200, description = "Successfully retrived list of Querent insights.", body = Vec<InsightInfo>)
    )
)]
pub async fn list_insights() -> Result<Vec<InsightInfo>, InsightError> {
	Ok(all_insights_info_available().await)
}

/// list insights handler.
pub fn list_insights_handler(
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("insights" / "installed")
		.and(warp::path::end())
		.and(warp::get())
		.then(list_insights)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
	post,
	tag = "Insights",
	path = "/insights/session",
	request_body = InsightAnalystRequest,
	responses(
		(status = 200, description = "Successfully started a insights session.", body = InsightAnalystResponse)
	),
)]
/// Start Insights Session
/// REST POST insights handler.
pub async fn start_insight_session_handler(
	request: InsightAnalystRequest,
	insights_service: Option<Arc<dyn InsightService>>,
) -> Result<InsightAnalystResponse, InsightError> {
	if insights_service.is_none() {
		return Err(InsightError::new(
			InsightErrorKind::NotSupported,
			Arc::new(anyhow::anyhow!("Insight service is not available")),
		));
	}
	let response = insights_service.unwrap().create_insight_session(request).await?;
	Ok(response)
}

pub fn start_insights_session_filter(
	insight_service: Option<Arc<dyn InsightService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("insights" / "session")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(insight_service)))
		.then(start_insight_session_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
    post,
    tag = "Insights",
    path = "/insights/prompt",
    request_body = InsightQuery,
    responses(
        (status = 200, description = "Successful query response.", body = InsightQueryResponse)
    ),
)]
/// Generate Deeper Insights (POST Variant)
///
/// REST POST insights handler.
///
/// Parses the insights request from the request body.
pub async fn insights_prompt_handler(
	request: InsightQuery,
	insight_service: Option<Arc<dyn InsightService>>,
) -> Result<InsightQueryResponse, InsightError> {
	if insight_service.is_none() {
		return Err(InsightError::new(
			InsightErrorKind::NotSupported,
			Arc::new(anyhow::anyhow!("Insight service is not available")),
		));
	}
	let response = insight_service.unwrap().provide_insight_input(request).await?;
	Ok(response)
}

pub fn insights_prompt_filter(
	insight_service: Option<Arc<dyn InsightService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("insights" / "prompt")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(insight_service)))
		.then(insights_prompt_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
	post,
	tag = "Insights",
	path = "/insights/session/stop",
	request_body = StopInsightSessionRequest,
	responses(
		(status = 200, description = "Successfully stopped the insights session.", body = StopInsightSessionResponse)
	),
)]
/// Stop Insights Session
/// REST POST insights handler.
pub async fn stop_insight_session_handler(
	request: StopInsightSessionRequest,
	insight_service: Option<Arc<dyn InsightService>>,
) -> Result<StopInsightSessionResponse, InsightError> {
	if insight_service.is_none() {
		return Err(InsightError::new(
			InsightErrorKind::NotSupported,
			Arc::new(anyhow::anyhow!("Insight service is not available")),
		));
	}
	let response = insight_service.unwrap().stop_insight_session(request).await?;
	Ok(response)
}

pub fn stop_insight_session_filter(
	insight_service: Option<Arc<dyn InsightService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("insights" / "session" / "stop")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(insight_service)))
		.then(stop_insight_session_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
	get,
	tag = "Insights",
	path = "/insights/list",
	responses(
		(status = 200, description = "Get pipelines metadata", body = InsightRequestInfoList)
	),
)]

/// Get pipelines metadata
pub async fn get_pipelines_history(
	insight_service: Option<Arc<dyn InsightService>>,
) -> Result<proto::insights::InsightRequestInfoList, InsightError> {
	if insight_service.is_none() {
		return Err(InsightError::new(
			InsightErrorKind::NotSupported,
			Arc::new(anyhow::anyhow!("Insight service is not available")),
		));
	}
	let response = insight_service.unwrap().get_insight_request_list().await?;
	Ok(response)
}

pub fn get_insights_history_handler(
	insight_service: Option<Arc<dyn InsightService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("insights" / "list")
		.and(warp::get())
		.and(require(Some(insight_service)))
		.then(get_pipelines_history)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}
