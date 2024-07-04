use insights::{
	all_insights_info_available, CustomInsightOption, InsightCustomOptionValue, InsightError,
	InsightErrorKind, InsightInfo,
};
use proto::{
	InsightAnalystRequest, InsightAnalystResponse, InsightQuery, InsightQueryResponse,
	StopInsightSessionRequest, StopInsightSessionResponse,
};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc};
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
		insights_get_handler,
		stop_insight_session_handler,
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
	),)
)]
pub struct InsightsApi;

/// list insights handler.
pub fn list_insights_handler(
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("insights")
		.and(warp::path::end())
		.and(warp::get())
		.then(list_insights)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}

#[utoipa::path(
    get,
    tag = "Insights",
    path = "/insights",
    responses(
        (status = 200, description = "Successfully retrived list of Querent insights.", body = Vec<InsightInfo>)
    )
)]
pub async fn list_insights() -> Result<Vec<InsightInfo>, Infallible> {
	Ok(all_insights_info_available().await)
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
	let response = insights_service.unwrap().start_insight_session(request).await?;
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
	let response = insight_service.unwrap().send_input(request).await?;
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
}
#[utoipa::path(
    get,
    tag = "Insights",
    path = "/insights/prompt",
    responses(
        (status = 200, description = "Successfully query response.", body = InsightQueryResponse)
    ),
    params(
        InsightPromptParam,
	)
)]
/// Generate Deeper Insights (GET Variant)
///
/// REST GET insights handler.
pub async fn insights_get_handler(
	request: InsightPromptParam,
	insight_service: Option<Arc<dyn InsightService>>,
) -> Result<InsightQueryResponse, InsightError> {
	if insight_service.is_none() {
		return Err(InsightError::new(
			InsightErrorKind::NotSupported,
			Arc::new(anyhow::anyhow!("Insight service is not available")),
		));
	}
	let request_required = InsightQuery { session_id: request.session_id, query: request.query };
	let response = insight_service.unwrap().send_input(request_required).await?;
	Ok(response)
}

pub fn insights_get_filter(
	insight_service: Option<Arc<dyn InsightService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("insights" / "prompt")
		.and(warp::query::<InsightPromptParam>())
		.and(warp::get())
		.and(require(Some(insight_service)))
		.then(insights_get_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}

#[derive(
	Debug, Default, Eq, PartialEq, Serialize, Deserialize, utoipa::IntoParams, utoipa::ToSchema,
)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub struct InsightPromptParam {
	/// The session id to use.
	pub session_id: String,
	/// The query to search for.
	pub query: String,
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
}
