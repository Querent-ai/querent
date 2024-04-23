use std::{collections::HashMap, sync::Arc};

use proto::semantics::CollectorConfig;

use crate::{Protocol, Source, SourceError, SourceErrorKind, SourceFactory, Uri};

pub struct SourceResolver {
	source_factories: HashMap<Protocol, Arc<dyn SourceFactory>>,
}

impl SourceResolver {
	pub fn new() -> Self {
		let mut source_factories = HashMap::new();

		SourceResolver { source_factories }
	}

	pub async fn resolve(
		&self,
		uri: &Uri,
		config: &CollectorConfig,
	) -> Result<Option<Arc<dyn Source>>, SourceError> {
		if uri.protocol.is_not_supported() {
			return Err(SourceError::new(
				SourceErrorKind::NotSupported,
				anyhow::anyhow!(
					"Backend in the config does not match the backend the protocol".to_string()
				)
				.into(),
			));
		}

		if let Some(factory) = self.source_factories.get(&uri.protocol) {
			Ok(factory.resolve(uri, config).await?)
		} else {
			Err(SourceError::new(
				SourceErrorKind::NotSupported,
				anyhow::anyhow!("No factory found for the backend".to_string()).into(),
			))
		}
	}
}
