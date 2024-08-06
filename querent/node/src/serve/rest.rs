use std::{net::SocketAddr, sync::Arc};

use common::{BoxFutureInfaillible, ServiceErrorCode};
use hyper::{http, http::HeaderValue, Method, Response, StatusCode};
use tower::{make::Shared, ServiceBuilder};
use tower_http::{
	compression::{
		predicate::{DefaultPredicate, Predicate, SizeAbove},
		CompressionLayer,
	},
	cors::CorsLayer,
};
use tracing::{error, info};
use utoipa_swagger_ui::Config;
use warp::{
	filters::path::{FullPath, Tail},
	http::Uri,
	redirect, Filter, Rejection, Reply,
};

use crate::{
	cluster_api::cluster_handler,
	delete_collectors_delete_handler,
	discovery_api::{
		discover_get_filter, discover_post_filter, get_discovery_history_handler,
		start_discovery_session_filter, stop_discovery_session_filter,
	},
	get_pipelines_history_handler, get_pipelines_metadata_handler,
	health_check_api::health_check_handlers,
	ingest_token_handler, ingest_tokens_put_handler,
	insight_api::rest::{
		get_insights_history_handler, insights_prompt_filter, list_insights_handler,
		start_insights_session_filter, stop_insight_session_filter,
	},
	json_api_response::{ApiError, JsonApiResponse},
	list_collectors_list_handler, metrics_handler, node_info_handler, observe_pipeline_get_handler,
	pipelines_get_all_handler, restart_pipeline_post_handler, set_collectors_post_handler,
	start_pipeline_post_handler, stop_pipeline_delete_handler, BodyFormat, BuildInfo,
	QuerentServices, RuntimeInfo,
};

/// The minimum size a response body must be in order to
/// be automatically compressed with gzip.
const MINIMUM_RESPONSE_COMPRESSION_SIZE: u16 = 10 << 10;

#[derive(Debug)]
pub(crate) struct InvalidJsonRequest(pub serde_json::Error);

impl warp::reject::Reject for InvalidJsonRequest {}

#[derive(Debug)]
pub(crate) struct InvalidArgument(pub String);

impl warp::reject::Reject for InvalidArgument {}

async fn serve_swagger(
	full_path: FullPath,
	tail: Tail,
	config: Arc<utoipa_swagger_ui::Config<'static>>,
) -> Result<Box<dyn Reply + 'static>, Rejection> {
	if full_path.as_str() == "/swagger-ui" {
		return Ok(Box::new(warp::redirect::found(Uri::from_static("/swagger-ui/"))));
	}

	let path = tail.as_str();
	match utoipa_swagger_ui::serve(path, config) {
		Ok(file) =>
			if let Some(file) = file {
				Ok(Box::new(
					Response::builder().header("Content-Type", file.content_type).body(file.bytes),
				))
			} else {
				Ok(Box::new(StatusCode::NOT_FOUND))
			},
		Err(error) => Ok(Box::new(
			Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(error.to_string()),
		)),
	}
}

/// Starts REST services.
pub(crate) async fn start_rest_server(
	rest_listen_addr: SocketAddr,
	services: Arc<QuerentServices>,
	readiness_trigger: BoxFutureInfaillible<()>,
	shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<()> {
	let request_counter = warp::log::custom(|_| {
		crate::SERVE_METRICS.http_requests_total.inc();
	});
	// Docs routes
	let api_doc = warp::path("api-doc.json")
		.and(warp::get())
		.map(|| warp::reply::json(&crate::openapi::build_docs()));
	let config = Arc::new(Config::from("/api-doc.json"));
	let swagger_ui = warp::path("swagger-ui")
		.and(warp::get())
		.and(warp::path::full())
		.and(warp::path::tail())
		.and(warp::any().map(move || config.clone()))
		.and_then(serve_swagger);

	// `/health/*` routes.
	let health_check_routes = health_check_handlers(
		services.cluster.clone(),
		Some(services.semantic_service_bus.clone()),
		services.secret_store.clone(),
	);

	// `/metrics` route.
	let metrics_routes = warp::path("metrics").and(warp::get()).map(metrics_handler);

	// `/api/v1/*` routes.
	let api_v1_root_route = api_v1_routes(services.clone());

	let redirect_root_to_ui_route = warp::path::end()
		.and(warp::get())
		.map(|| redirect(http::Uri::from_static("/ui/workflows")));

	let extra_headers =
		warp::reply::with::headers(services.node_config.rest_config.extra_headers.clone());

	// Combine all the routes together.
	let rest_routes = api_v1_root_route
		.or(api_doc)
		.or(swagger_ui)
		.or(redirect_root_to_ui_route)
		.or(health_check_routes)
		.or(metrics_routes)
		.with(request_counter)
		.recover(recover_fn)
		.with(extra_headers)
		.boxed();

	let warp_service = warp::service(rest_routes);
	let compression_predicate =
		DefaultPredicate::new().and(SizeAbove::new(MINIMUM_RESPONSE_COMPRESSION_SIZE));
	let cors = build_cors(&services.node_config.rest_config.cors_allow_origins);

	let service = ServiceBuilder::new()
		.layer(CompressionLayer::new().gzip(true).compress_when(compression_predicate))
		.layer(cors)
		.service(warp_service);

	info!(
		rest_listen_addr=?rest_listen_addr,
		"Starting REST server listening on {rest_listen_addr}."
	);

	// `graceful_shutdown()` seems to be blocking in presence of existing connections.
	// The following approach of dropping the serve supposedly is not bullet proof, but it seems to
	// work in our unit test.
	//
	// See more of the discussion here:
	// https://github.com/hyperium/hyper/issues/2386
	let serve_fut = async move {
		tokio::select! {
			 res = hyper::Server::bind(&rest_listen_addr).serve(Shared::new(service)) => { res }
			 _ = shutdown_signal => { Ok(()) }
		}
	};

	let (serve_res, _trigger_res) = tokio::join!(serve_fut, readiness_trigger);
	serve_res?;
	Ok(())
}

fn api_v1_routes(
	services: Arc<QuerentServices>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	if !services.node_config.rest_config.extra_headers.is_empty() {
		info!(
			"Extra headers will be added to all responses: {:?}",
			services.node_config.rest_config.extra_headers
		);
	}
	let api_v1_root_url = warp::path!("api" / "v1" / ..);
	api_v1_root_url
		.and(
			cluster_handler(services.cluster.clone())
				.or(node_info_handler(
					BuildInfo::get(),
					RuntimeInfo::get(),
					Arc::new(services.node_config.clone()),
				))
				.or(pipelines_get_all_handler(Some(services.semantic_service_bus.clone())))
				.or(observe_pipeline_get_handler(Some(services.semantic_service_bus.clone())))
				.or(start_pipeline_post_handler(
					Some(services.semantic_service_bus.clone()),
					services.event_storages.clone(),
					services.index_storages.clone(),
					services.secret_store.clone(),
					services.metadata_store.clone(),
				))
				.or(get_pipelines_metadata_handler(Some(services.semantic_service_bus.clone())))
				.or(stop_pipeline_delete_handler(Some(services.semantic_service_bus.clone())))
				.or(ingest_token_handler(Some(services.semantic_service_bus.clone())))
				.or(ingest_tokens_put_handler(Some(services.semantic_service_bus.clone())))
				.or(restart_pipeline_post_handler(Some(services.semantic_service_bus.clone())))
				.or(start_discovery_session_filter(services.discovery_service.clone()))
				.or(discover_get_filter(services.discovery_service.clone()))
				.or(discover_post_filter(services.discovery_service.clone()))
				.or(stop_discovery_session_filter(services.discovery_service.clone()))
				.or(set_collectors_post_handler(services.secret_store.clone()))
				.or(delete_collectors_delete_handler(services.secret_store.clone()))
				.or(list_collectors_list_handler(services.secret_store.clone()))
				.or(start_insights_session_filter(services.insight_service.clone()))
				.or(stop_insight_session_filter(services.insight_service.clone()))
				.or(insights_prompt_filter(services.insight_service.clone()))
				.or(list_insights_handler())
				.or(get_pipelines_history_handler(services.metadata_store.clone()))
				.or(get_insights_history_handler(services.insight_service.clone()))
				.or(get_discovery_history_handler(services.discovery_service.clone())),
		)
		.boxed()
}

/// This function returns a formatted error based on the given rejection reason.
/// The ordering of rejection processing is very important, we need to start

// More on this here: https://github.com/seanmonstar/warp/issues/388.
// We may use this work on the PR is merged: https://github.com/seanmonstar/warp/pull/909.
pub async fn recover_fn(rejection: Rejection) -> Result<impl Reply, Rejection> {
	let err = get_status_with_error(rejection);
	let status_code = err.service_code.to_http_status_code();
	Ok(JsonApiResponse::new::<(), _>(&Err(err), status_code, &BodyFormat::default()))
}

fn get_status_with_error(rejection: Rejection) -> ApiError {
	if rejection.is_not_found() {
		ApiError {
			service_code: ServiceErrorCode::NotFound,
			message: "Route not found".to_string(),
		}
	} else if let Some(error) = rejection.find::<serde_qs::Error>() {
		ApiError { service_code: ServiceErrorCode::BadRequest, message: error.to_string() }
	} else if let Some(error) = rejection.find::<InvalidJsonRequest>() {
		// Happens when the request body could not be deserialized correctly.
		ApiError { service_code: ServiceErrorCode::BadRequest, message: error.0.to_string() }
	} else if let Some(error) = rejection.find::<InvalidArgument>() {
		// Happens when the url path or request body contains invalid argument(s).
		ApiError { service_code: ServiceErrorCode::BadRequest, message: error.0.to_string() }
	} else if let Some(error) = rejection.find::<warp::filters::body::BodyDeserializeError>() {
		// Happens when the request body could not be deserialized correctly.
		ApiError { service_code: ServiceErrorCode::BadRequest, message: error.to_string() }
	} else if let Some(error) = rejection.find::<warp::reject::UnsupportedMediaType>() {
		ApiError {
			service_code: ServiceErrorCode::UnsupportedMediaType,
			message: error.to_string(),
		}
	} else if let Some(error) = rejection.find::<warp::reject::InvalidQuery>() {
		ApiError { service_code: ServiceErrorCode::BadRequest, message: error.to_string() }
	} else if let Some(error) = rejection.find::<warp::reject::LengthRequired>() {
		ApiError { service_code: ServiceErrorCode::BadRequest, message: error.to_string() }
	} else if let Some(error) = rejection.find::<warp::reject::MissingHeader>() {
		ApiError { service_code: ServiceErrorCode::BadRequest, message: error.to_string() }
	} else if let Some(error) = rejection.find::<warp::reject::InvalidHeader>() {
		ApiError { service_code: ServiceErrorCode::BadRequest, message: error.to_string() }
	} else if let Some(error) = rejection.find::<warp::reject::MethodNotAllowed>() {
		ApiError { service_code: ServiceErrorCode::MethodNotAllowed, message: error.to_string() }
	} else if let Some(error) = rejection.find::<warp::reject::PayloadTooLarge>() {
		ApiError { service_code: ServiceErrorCode::BadRequest, message: error.to_string() }
	} else {
		error!("REST server error: {:?}", rejection);
		ApiError {
			service_code: ServiceErrorCode::Internal,
			message: "internal server error".to_string(),
		}
	}
}

fn build_cors(cors_origins: &[String]) -> CorsLayer {
	let mut cors = CorsLayer::new().allow_methods([
		Method::GET,
		Method::POST,
		Method::PUT,
		Method::DELETE,
		Method::OPTIONS,
	]);
	if !cors_origins.is_empty() {
		let allow_any = cors_origins.iter().any(|origin| origin.as_str() == "*");

		if allow_any {
			info!("CORS is enabled, all origins will be allowed");
			cors = cors.allow_origin(tower_http::cors::Any);
		} else {
			info!(origins = ?cors_origins, "CORS is enabled, the following origins will be allowed");
			let origins = cors_origins
				.iter()
				.map(|origin| origin.parse::<HeaderValue>().unwrap())
				.collect::<Vec<_>>();
			cors = cors.allow_origin(origins);
		};
	}

	cors
}
