use std::sync::Arc;

use async_trait::async_trait;
use proto::semantics::{Backend, CollectorConfig};

use crate::{Source, SourceError, Uri};

#[async_trait]
pub trait SourceFactory: Send + Sync {
	fn backend(&self) -> Backend;

	async fn resolve(
		&self,
		uri: &Uri,
		config: &CollectorConfig,
	) -> Result<Option<Arc<dyn Source>>, SourceError>;
}

// Define an enum for your custom errors
#[derive(Debug)]
pub struct UnsupportedSource {
	backend: Backend,
	message: String,
}

#[async_trait]
impl SourceFactory for UnsupportedSource {
	fn backend(&self) -> Backend {
		self.backend.clone()
	}

	async fn resolve(
		&self,
		_uri: &Uri,
		_config: &CollectorConfig,
	) -> Result<Option<Arc<dyn Source>>, SourceError> {
		let err: SourceError = SourceError {
			kind: crate::SourceErrorKind::NotSupported,
			source: Arc::new(anyhow::anyhow!("{}", self.message)),
		};

		Err(err)
	}
}
