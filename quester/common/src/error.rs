use std::convert::Infallible;

/// This enum serves as a Rosetta Stone of
/// gRPC and HTTP status code.
///
/// It is voluntarily a restricted subset.
#[derive(Clone, Copy)]
pub enum ServiceErrorCode {
	AlreadyExists,
	BadRequest,
	Internal,
	MethodNotAllowed,
	NotFound,
	// Used for APIs that are available in Elasticsearch but not available yet in Quickwit.
	NotSupportedYet,
	RateLimited,
	Timeout,
	Unavailable,
	UnsupportedMediaType,
}

impl ServiceErrorCode {
	pub fn to_grpc_status_code(self) -> tonic::Code {
		match self {
			ServiceErrorCode::AlreadyExists => tonic::Code::AlreadyExists,
			ServiceErrorCode::BadRequest => tonic::Code::InvalidArgument,
			ServiceErrorCode::Internal => tonic::Code::Internal,
			ServiceErrorCode::MethodNotAllowed => tonic::Code::InvalidArgument,
			ServiceErrorCode::NotFound => tonic::Code::NotFound,
			ServiceErrorCode::NotSupportedYet => tonic::Code::Unimplemented,
			ServiceErrorCode::RateLimited => tonic::Code::ResourceExhausted,
			ServiceErrorCode::Timeout => tonic::Code::DeadlineExceeded,
			ServiceErrorCode::Unavailable => tonic::Code::Unavailable,
			ServiceErrorCode::UnsupportedMediaType => tonic::Code::InvalidArgument,
		}
	}
	pub fn to_http_status_code(self) -> http::StatusCode {
		match self {
			ServiceErrorCode::AlreadyExists => http::StatusCode::BAD_REQUEST,
			ServiceErrorCode::BadRequest => http::StatusCode::BAD_REQUEST,
			ServiceErrorCode::Internal => http::StatusCode::INTERNAL_SERVER_ERROR,
			ServiceErrorCode::MethodNotAllowed => http::StatusCode::METHOD_NOT_ALLOWED,
			ServiceErrorCode::NotFound => http::StatusCode::NOT_FOUND,
			ServiceErrorCode::NotSupportedYet => http::StatusCode::NOT_IMPLEMENTED,
			ServiceErrorCode::RateLimited => http::StatusCode::TOO_MANY_REQUESTS,
			ServiceErrorCode::Unavailable => http::StatusCode::SERVICE_UNAVAILABLE,
			ServiceErrorCode::UnsupportedMediaType => http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
			ServiceErrorCode::Timeout => http::StatusCode::REQUEST_TIMEOUT,
		}
	}
}

pub trait ServiceError: ToString {
	fn grpc_error(&self) -> tonic::Status {
		let grpc_code = self.error_code().to_grpc_status_code();
		let error_msg = self.to_string();
		tonic::Status::new(grpc_code, error_msg)
	}

	fn error_code(&self) -> ServiceErrorCode;
}

impl ServiceError for Infallible {
	fn error_code(&self) -> ServiceErrorCode {
		unreachable!()
	}
}

pub fn convert_to_grpc_result<T, E: ServiceError>(
	res: Result<T, E>,
) -> Result<tonic::Response<T>, tonic::Status> {
	res.map(tonic::Response::new).map_err(|error| error.grpc_error())
}
