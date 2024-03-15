use std::sync::Arc;

use async_trait::async_trait;
use futures::TryStreamExt;
use proto::{
	error::convert_to_grpc_result, semantics::semantics_service_grpc_server as grpc,
	GrpcServiceError,
};
use tracing::instrument;
