use diesel::result::{ConnectionError, ConnectionResult};
use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager, ManagerConfig},
};
use futures_util::{future::BoxFuture, FutureExt};
use std::{
	sync::Arc,
	time::{Duration, SystemTime},
};
use tracing::error;

use rustls::{
	client::{ServerCertVerified, ServerCertVerifier},
	ServerName,
};

use crate::{StorageError, StorageErrorKind, StorageResult};
use deadpool::Runtime;

pub type ActualDbPool = Pool<AsyncPgConnection>;
pub const FETCH_LIMIT_MAX: i64 = 50;
pub const SITEMAP_LIMIT: i64 = 50000;
pub const SITEMAP_DAYS: i64 = 31;
const POOL_TIMEOUT: Option<Duration> = Some(Duration::from_secs(5));

pub struct PostgresStorage {
	pub pool: ActualDbPool,
	pub db_url: String,
}

struct NoCertVerifier {}

impl ServerCertVerifier for NoCertVerifier {
	fn verify_server_cert(
		&self,
		_end_entity: &rustls::Certificate,
		_intermediates: &[rustls::Certificate],
		_server_name: &ServerName,
		_scts: &mut dyn Iterator<Item = &[u8]>,
		_ocsp_response: &[u8],
		_now: SystemTime,
	) -> Result<ServerCertVerified, rustls::Error> {
		// Will verify all (even invalid) certs without any checks (sslmode=require)
		Ok(ServerCertVerified::assertion())
	}
}

impl PostgresStorage {
	pub async fn new(db_url: &str) -> StorageResult<Self> {
		let tls_enabled = db_url.contains("sslmode=require");
		let manager = if tls_enabled {
			// diesel-async does not support any TLS connections out of the box, so we need to manually
			// provide a setup function which handles creating the connection
			let mut config = ManagerConfig::default();
			config.custom_setup = Box::new(establish_connection);
			AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(db_url, config)
		} else {
			AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_url)
		};
		let pool = Pool::builder(manager)
			.max_size(10)
			.wait_timeout(POOL_TIMEOUT)
			.create_timeout(POOL_TIMEOUT)
			.recycle_timeout(POOL_TIMEOUT)
			.runtime(Runtime::Tokio1)
			.build()
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		Ok(PostgresStorage { pool, db_url: db_url.to_string() })
	}
}

fn establish_connection(config: &str) -> BoxFuture<ConnectionResult<AsyncPgConnection>> {
	let fut = async {
		let rustls_config = rustls::ClientConfig::builder()
			.with_safe_defaults()
			.with_custom_certificate_verifier(Arc::new(NoCertVerifier {}))
			.with_no_client_auth();

		let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
		let (client, conn) = tokio_postgres::connect(config, tls)
			.await
			.map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
		tokio::spawn(async move {
			if let Err(e) = conn.await {
				error!("Database connection failed: {e}");
			}
		});
		AsyncPgConnection::try_from(client).await
	};
	fut.boxed()
}
