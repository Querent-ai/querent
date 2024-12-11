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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use async_trait::async_trait;
use common::{CollectedBytes, Retryable};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::{
	fmt::{self, Debug},
	io,
	ops::Range,
	path::Path,
	pin::Pin,
	sync::Arc,
};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};

pub trait SendableAsync: AsyncWrite + Send + Unpin {}
impl<W: AsyncWrite + Send + Unpin> SendableAsync for W {}

use crate::default_copy_to_file;

/// Storage error kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SourceErrorKind {
	/// Connection error.
	Connection,
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
	/// Service error.
	Service,
	/// Internal error.
	Internal,
}

/// Generic SourceError.
#[derive(Debug, Clone, Error)]
#[error("source error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct SourceError {
	pub kind: SourceErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// Generic Result type for source type operations.
pub type SourceResult<T> = Result<T, SourceError>;

impl SourceError {
	pub fn new(kind: SourceErrorKind, source: Arc<anyhow::Error>) -> Self {
		SourceError { kind, source }
	}

	/// Add some context to the wrapper error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		SourceError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the corresponding `SourceErrorKind` for this error.
	pub fn kind(&self) -> SourceErrorKind {
		self.kind
	}
}

impl From<io::Error> for SourceError {
	fn from(err: io::Error) -> SourceError {
		match err.kind() {
			io::ErrorKind::NotFound =>
				SourceError::new(SourceErrorKind::NotFound, Arc::new(err.into())),
			_ => SourceError::new(SourceErrorKind::Io, Arc::new(err.into())),
		}
	}
}

impl From<serde_json::Error> for SourceError {
	fn from(err: serde_json::Error) -> SourceError {
		SourceError::new(SourceErrorKind::Io, Arc::new(err.into()))
	}
}

impl From<reqwest::Error> for SourceError {
	fn from(err: reqwest::Error) -> Self {
		SourceError::new(
			SourceErrorKind::Io,
			Arc::new(anyhow::anyhow!("Error while converting the request into struct: {:?}", err)),
		)
	}
}

impl Retryable for SourceError {
	fn is_retryable(&self) -> bool {
		match self.kind {
			SourceErrorKind::Connection | SourceErrorKind::Polling => true,
			_ => false,
		}
	}
}

impl From<google_drive3::Error> for SourceError {
	fn from(err: google_drive3::Error) -> Self {
		SourceError::new(
			SourceErrorKind::Connection,
			Arc::new(anyhow::anyhow!("Error while converting the request into struct: {:?}", err)),
		)
	}
}

impl From<SourceError> for google_drive3::Error {
	fn from(_err: SourceError) -> Self {
		google_drive3::Error::FieldClash("Error in SourceError")
	}
}

/// Sources is all possible data sources that can be used to create a `CollectedBytes`.
#[async_trait]
pub trait DataSource: fmt::Debug + Send + Sync {
	/// Establishes a connection to the source.
	async fn check_connectivity(&self) -> anyhow::Result<()>;

	/// Pulls data from the source and copies it to a file.
	async fn copy_to_file(&self, path: &Path, output_path: &Path) -> SourceResult<u64> {
		default_copy_to_file(self, path, output_path).await
	}

	/// Downloads a slice of data from the source.
	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>>;

	/// Downloads a slice of data from the source as a stream.
	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>>;

	/// Downloads the entire content of a "small" file, returns an in memory buffer.
	/// For large files prefer `copy_to_file`.
	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>>;

	async fn exists(&self, path: &Path) -> SourceResult<bool> {
		match self.file_num_bytes(path).await {
			Ok(_) => Ok(true),
			Err(storage_err) if storage_err.kind() == SourceErrorKind::NotFound => Ok(false),
			Err(other_storage_err) => Err(other_storage_err),
		}
	}

	/// Returns a file size.
	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64>;

	fn copy_to<'life0, 'life1, 'life2, 'async_trait>(
		&'life0 self,
		path: &'life1 Path,
		output: &'life2 mut dyn SendableAsync,
	) -> ::core::pin::Pin<
		Box<
			dyn ::core::future::Future<Output = SourceResult<()>>
				+ ::core::marker::Send
				+ 'async_trait,
		>,
	>
	where
		'life0: 'async_trait,
		'life1: 'async_trait,
		'life2: 'async_trait,
		Self: 'async_trait;

	/// Polls data from the source and sends it to the output.
	/// Output is a sender that can be used to send data to the next actor in the pipeline.
	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>>;
}
