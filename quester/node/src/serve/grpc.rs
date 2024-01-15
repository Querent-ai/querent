use std::{net::SocketAddr, sync::Arc};

use common::BoxFutureInfaillible;
use tonic::transport::Server;

use crate::QuesterServices;

/// Starts and binds gRPC services to `grpc_listen_addr`.
pub(crate) async fn _start_grpc_server(
	_grpc_listen_addr: SocketAddr,
	_services: Arc<QuesterServices>,
	_readiness_trigger: BoxFutureInfaillible<()>,
	_shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<()> {
	let mut _server = Server::builder();
	// TODO
	Ok(())
}
