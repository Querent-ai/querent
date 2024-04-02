#![allow(clippy::derive_partial_eq_without_eq)]
#![deny(clippy::disallowed_methods)]
#![allow(rustdoc::invalid_html_tags)]
pub mod cluster;
pub mod error;
pub use error::*;
pub mod config;
pub mod semantics;
pub mod types;
pub use config::*;
pub mod discovery;
use ::opentelemetry::{
	global,
	propagation::{Extractor, Injector},
};
pub use discovery::*;
use tonic::{service::Interceptor, Status};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Clone, Debug)]
pub struct SpanContextInterceptor;
pub struct MutMetadataMap<'a>(&'a mut tonic::metadata::MetadataMap);

impl Interceptor for SpanContextInterceptor {
	fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
		global::get_text_map_propagator(|propagator| {
			propagator.inject_context(
				&tracing::Span::current().context(),
				&mut MutMetadataMap(request.metadata_mut()),
			)
		});
		Ok(request)
	}
}

impl<'a> Injector for MutMetadataMap<'a> {
	/// Sets a key-value pair in the [`MetadataMap`]. No-op if the key or value is invalid.
	fn set(&mut self, key: &str, value: String) {
		if let Ok(metadata_key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
			if let Ok(metadata_value) = tonic::metadata::MetadataValue::try_from(&value) {
				self.0.insert(metadata_key, metadata_value);
			}
		}
	}
}

impl<'a> Extractor for MutMetadataMap<'a> {
	/// Gets a value for a key from the MetadataMap.  If the value can't be converted to &str,
	/// returns None.
	fn get(&self, key: &str) -> Option<&str> {
		self.0.get(key).and_then(|metadata| metadata.to_str().ok())
	}

	/// Collect all the keys from the MetadataMap.
	fn keys(&self) -> Vec<&str> {
		self.0
			.keys()
			.map(|key| match key {
				tonic::metadata::KeyRef::Ascii(v) => v.as_str(),
				tonic::metadata::KeyRef::Binary(v) => v.as_str(),
			})
			.collect::<Vec<_>>()
	}
}
