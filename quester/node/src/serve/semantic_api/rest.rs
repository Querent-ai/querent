use std::convert::Infallible;

use actors::{AskError, MessageBus, Observe};
use querent::{SemanticService, SemanticServiceCounters};
use warp::{reject::Rejection, Filter};

use crate::{extract_format_from_qs, make_json_api_response, serve::require};

#[derive(utoipa::OpenApi)]
#[openapi(paths(semantic_endpoint))]
pub struct SemanticApi;

#[utoipa::path(
    get,
    tag = "Semantic Service",
    path = "/semantic",
    responses(
        (status = 200, description = "Successfully observed semantic pipelines.", body = IndexingStatistics)
    ),
)]
/// Observe Indexing Pipeline
async fn semantic_endpoint(
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<SemanticServiceCounters, AskError<Infallible>> {
	let counters = semantic_service_mailbox.ask(Observe).await?;
	semantic_service_mailbox.ask(Observe).await?;
	Ok(counters)
}

fn pipelines_get_filter() -> impl Filter<Extract = (), Error = Rejection> + Clone {
	warp::path!("semantic").and(warp::get())
}

pub fn pipelines_get_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	pipelines_get_filter()
		.and(require(semantic_service_bus))
		.then(semantic_endpoint)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}
