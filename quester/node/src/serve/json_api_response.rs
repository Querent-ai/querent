use common::{ServiceError, ServiceErrorCode};
use hyper::{
	header::CONTENT_TYPE,
	http::{status, HeaderValue},
	Body, Response,
};
use serde::{self, Serialize};
use warp::Reply;

use super::BodyFormat;

const JSON_SERIALIZATION_ERROR: &str = "JSON serialization failed.";

#[derive(Serialize)]
pub(crate) struct ApiError {
	// For now, we want to keep ApiError as simple as possible
	// and return just a message.
	#[serde(skip_serializing)]
	pub service_code: ServiceErrorCode,
	pub message: String,
}

impl ServiceError for ApiError {
	fn error_code(&self) -> ServiceErrorCode {
		self.service_code
	}
}

impl ToString for ApiError {
	fn to_string(&self) -> String {
		self.message.clone()
	}
}

/// Makes a JSON API response from a result.
/// The error is wrapped into an [`ApiError`] to publicly expose
/// a consistent error format.
pub(crate) fn make_json_api_response<T: serde::Serialize, E: ServiceError>(
	result: Result<T, E>,
	format: BodyFormat,
) -> JsonApiResponse {
	let result_with_api_error =
		result.map_err(|err| ApiError { service_code: err.error_code(), message: err.to_string() });
	let status_code = match &result_with_api_error {
		Ok(_) => status::StatusCode::OK,
		Err(err) => err.error_code().to_http_status_code(),
	};
	JsonApiResponse::new(&result_with_api_error, status_code, &format)
}

/// A JSON reply for the REST API.
pub struct JsonApiResponse {
	status_code: status::StatusCode,
	inner: Result<Vec<u8>, ()>,
}

impl JsonApiResponse {
	pub fn new<T: serde::Serialize, E: serde::Serialize>(
		result: &Result<T, E>,
		status_code: status::StatusCode,
		body_format: &BodyFormat,
	) -> Self {
		let inner = body_format.result_to_vec(result);
		JsonApiResponse { status_code, inner }
	}
}

impl Reply for JsonApiResponse {
	#[inline]
	fn into_response(self) -> Response<Body> {
		match self.inner {
			Ok(body) => {
				let mut response = Response::new(body.into());
				response
					.headers_mut()
					.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
				*response.status_mut() = self.status_code;
				response
			},
			Err(()) => warp::reply::json(&ApiError {
				service_code: ServiceErrorCode::Internal,
				message: JSON_SERIALIZATION_ERROR.to_string(),
			})
			.into_response(),
		}
	}
}
