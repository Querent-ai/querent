// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

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
use tonic::{service::Interceptor, Status};
use tracing_opentelemetry::OpenTelemetrySpanExt;
pub mod insights;
pub use insights::*;
pub mod layer;

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
