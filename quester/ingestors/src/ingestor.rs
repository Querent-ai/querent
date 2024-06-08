use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{pin_mut, Stream, StreamExt};
use querent_synapse::comm::IngestedTokens;
use serde::{Deserialize, Serialize};
use std::{fmt, io, pin::Pin, sync::Arc};
use thiserror::Error;

use crate::pdf::pdfv1::PdfIngestor;
use crate::txt::txt::TxtIngestor;
use crate::html::html::HtmlIngestor;

/// Ingestor error kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IngestorErrorKind {
	/// Polling error.
	Polling,
	/// Not supported error.
	NotSupported,
	/// Io error.
	Io,
	/// Not found error.
	NotFound,
	/// Unauthorized error.
	Unauthorized,
	/// Internal error.
	Internal,
}

/// Generic IngestorError.
#[derive(Debug, Clone, Error)]
#[error("source error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct IngestorError {
	pub kind: IngestorErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// Generic Result type for source type operations.
pub type IngestorResult<T> = Result<T, IngestorError>;

impl IngestorError {
	pub fn new(kind: IngestorErrorKind, source: Arc<anyhow::Error>) -> Self {
		IngestorError { kind, source }
	}

	/// Add some context to the wrapper error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		IngestorError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the corresponding `IngestorErrorKind` for this error.
	pub fn kind(&self) -> IngestorErrorKind {
		self.kind
	}
}

impl From<io::Error> for IngestorError {
	fn from(err: io::Error) -> IngestorError {
		match err.kind() {
			io::ErrorKind::NotFound =>
				IngestorError::new(IngestorErrorKind::NotFound, Arc::new(err.into())),
			_ => IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())),
		}
	}
}

impl From<serde_json::Error> for IngestorError {
	fn from(err: serde_json::Error) -> IngestorError {
		IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into()))
	}
}

// Define the trait for async processor
#[async_trait]
pub trait AsyncProcessor: Send + Sync {
	async fn process_text(&self, data: IngestedTokens) -> IngestorResult<IngestedTokens>;
}

// Define the trait for BaseIngestor
#[async_trait]
pub trait BaseIngestor: Send + Sync {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>);

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>;
}

// apply processors to stream of IngestedTokens and return a stream of IngestedTokens
pub async fn process_ingested_tokens_stream(
	ingested_tokens_stream: Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send>>,
	processors: Vec<Arc<dyn AsyncProcessor>>,
) -> Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send>> {
	let stream = stream! {
		pin_mut!(ingested_tokens_stream);
		while let Some(ingested_tokens_result) = ingested_tokens_stream.next().await {
			match ingested_tokens_result {
				Ok(ingested_tokens) => {
					let mut ingested_tokens = ingested_tokens;
					for processor in processors.iter() {
						match processor.process_text(ingested_tokens.clone()).await {
							Ok(processed_tokens) => {
								ingested_tokens = processed_tokens;
							},
							Err(e) => {
								yield Err(e);
							}
						}
					}
					yield Ok(ingested_tokens);
				},
				Err(e) => {
					yield Err(e);
				}
			}
		}
	};

	Box::pin(stream)
}

pub async fn resolve_ingestor_with_extension(
	extension: &str,
) -> IngestorResult<Arc<dyn BaseIngestor>> {
	match extension {
		"pdf" => Ok(Arc::new(PdfIngestor::new())),
		"txt" => Ok(Arc::new(TxtIngestor::new())),
		"html" => Ok(Arc::new(HtmlIngestor::new())),
		_ => Err(IngestorError::new(
			IngestorErrorKind::NotSupported,
			Arc::new(anyhow::anyhow!("Extension not supported")),
		)),
	}
}
