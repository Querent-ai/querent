pub mod aws;
use std::time::Duration;

pub use aws::*;
use aws_config::{
	retry::RetryConfig, stalled_stream_protection::StalledStreamProtectionConfig, BehaviorVersion,
};
pub use aws_smithy_async::rt::sleep::TokioSleep;
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;
use aws_types::region::Region;
use hyper::{client::HttpConnector, Client as HyperClient};
use hyper_rustls::HttpsConnectorBuilder;
use tokio::sync::OnceCell;
pub mod retry;

pub const DEFAULT_AWS_REGION: Region = Region::from_static("us-east-1");

/// Initialises and returns the AWS config.
pub async fn get_aws_config() -> &'static aws_config::SdkConfig {
	static SDK_CONFIG: OnceCell<aws_config::SdkConfig> = OnceCell::const_new();

	SDK_CONFIG
		.get_or_init(|| async {
			let mut http_connector = HttpConnector::new();
			http_connector.enforce_http(false); // Enforced by `HttpsConnector`.
			http_connector.set_nodelay(true);

			let https_connector = HttpsConnectorBuilder::new()
				.with_native_roots()
				.https_or_http()
				// We do not enable HTTP2.
				// It is not enabled on S3 and it does not seem to work with Google Cloud Storage at
				// this point. https://github.com/quickwit-oss/quickwit/issues/1584
				//
				// (HTTP2 would be awesome since we do a lot of concurrent requests and
				// HTTP2 enables multiplexing a given connection.)
				.enable_http1()
				.wrap_connector(http_connector);

			let mut hyper_client_builder = HyperClient::builder();
			hyper_client_builder.pool_idle_timeout(Duration::from_secs(30));
			let hyper_client = HyperClientBuilder::new()
				.hyper_builder(hyper_client_builder)
				.build(https_connector);

			aws_config::defaults(BehaviorVersion::v2024_03_28())
				.stalled_stream_protection(StalledStreamProtectionConfig::enabled().build())
				.http_client(hyper_client)
				// Currently handle this ourselves so probably best for now to leave it as is.
				.retry_config(RetryConfig::disabled())
				.sleep_impl(TokioSleep::default())
				.load()
				.await
		})
		.await
}
