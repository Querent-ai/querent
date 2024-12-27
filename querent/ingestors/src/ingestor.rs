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

use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{pin_mut, stream, Stream, StreamExt};
use proto::semantics::IngestedTokens;
use serde::{Deserialize, Serialize};
use std::{fmt, io, pin::Pin, sync::Arc};
use thiserror::Error;

use crate::{
	code::code::CodeIngestor, csv::csv::CsvIngestor, doc::doc::DocIngestor,
	docx::docx::DocxIngestor, html::html::HtmlIngestor, image::image::ImageIngestor,
	json::json::JsonIngestor, odp::odp::OdpIngestor, pdf::pdfv1::PdfIngestor,
	pptx::pptx::PptxIngestor, txt::txt::TxtIngestor, xlsx::xlsx::XlsxIngestor,
	xml::xml::XmlIngestor,
};
use tracing::info;

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
	/// Csv error,
	Csv,
	/// Zip error
	ZipError,
	/// Xml error
	Xml,
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

impl From<csv::Error> for IngestorError {
	fn from(err: csv::Error) -> IngestorError {
		IngestorError::new(IngestorErrorKind::Csv, Arc::new(err.into()))
	}
}

impl From<zip::result::ZipError> for IngestorError {
	fn from(error: zip::result::ZipError) -> Self {
		IngestorError::new(IngestorErrorKind::ZipError, Arc::new(error.into()))
	}
}

impl From<xml::reader::Error> for IngestorError {
	fn from(error: xml::reader::Error) -> Self {
		IngestorError::new(IngestorErrorKind::Xml, Arc::new(error.into()))
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

pub struct UnsupportedIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl UnsupportedIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
	}
}
#[async_trait]
impl BaseIngestor for UnsupportedIngestor {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
		self.processors = processors;
	}

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
	{
		let mut extension = String::new();
		for collected_bytes in all_collected_bytes {
			extension = collected_bytes.extension.unwrap().clone();
			break;
		}
		info!("The following extension is unsupported at the moment: {:?}", extension.clone());
		Ok(Box::pin(stream::empty()))
	}
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
	let programming_languages = vec![
		"py", "pyw", "pyp", "js", "mjs", "java", "cpp", "h", "hpp", "c", "h", "cs", "rb", "swift",
		"php", "php3", "php4", "php5", "phtml", "html", "htm", "css", "go", "rs", "kt", "ts", "pl",
		"sql", "r", "m", "sh", "bash", "zsh", "dart", "scala", "groovy", "lua", "m", "vb",
	];
	if programming_languages.contains(&extension) {
		return Ok(Arc::new(CodeIngestor::new()));
	}
	match extension {
		"pdf" => Ok(Arc::new(PdfIngestor::new())),
		"html" => Ok(Arc::new(HtmlIngestor::new())),
		"csv" => Ok(Arc::new(CsvIngestor::new())),
		"xml" => Ok(Arc::new(XmlIngestor::new())),
		"docx" => Ok(Arc::new(DocxIngestor::new())),
		"doc" => Ok(Arc::new(DocIngestor::new())),
		"jpeg" => Ok(Arc::new(ImageIngestor::new())),
		"jpg" => Ok(Arc::new(ImageIngestor::new())),
		"png" => Ok(Arc::new(ImageIngestor::new())),
		"json" => Ok(Arc::new(JsonIngestor::new())),
		"pptx" => Ok(Arc::new(PptxIngestor::new())),
		"odp" => Ok(Arc::new(OdpIngestor::new())),
		"xlsx" => Ok(Arc::new(XlsxIngestor::new())),
		"bmp" => Ok(Arc::new(ImageIngestor::new())),
		"gif" => Ok(Arc::new(ImageIngestor::new())),
		"hdr" => Ok(Arc::new(ImageIngestor::new())),
		"ico" => Ok(Arc::new(ImageIngestor::new())),
		"exr" => Ok(Arc::new(ImageIngestor::new())),
		"pnm" => Ok(Arc::new(ImageIngestor::new())),
		"qoi" => Ok(Arc::new(ImageIngestor::new())),
		"tiff" => Ok(Arc::new(ImageIngestor::new())),
		"webp" => Ok(Arc::new(ImageIngestor::new())),
		"ff" => Ok(Arc::new(ImageIngestor::new())),
		"dds" => Ok(Arc::new(ImageIngestor::new())),
		"news" | "email" | "notion" | "txt" | "" | "md" | "slack" | "jira" =>
			Ok(Arc::new(TxtIngestor::new())),
		_ => Ok(Arc::new(UnsupportedIngestor::new())),
	}
}
