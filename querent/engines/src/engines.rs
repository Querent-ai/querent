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
use common::EventState;
use futures::Stream;
use llms::LLMError;
use proto::semantics::IngestedTokens;
use serde::{Deserialize, Serialize};
use std::{fmt, io, pin::Pin, sync::Arc};
use thiserror::Error;
use tokio::sync::mpsc::Receiver;

use candle_core::Error as CandleCoreError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EngineErrorKind {
	/// Event streaming failed
	EventStream,
	/// Io error.
	Io,
	/// Not found error.
	NotFound,
	/// Model error.
	ModelError, // <-- Add this new variant
}

/// Generic IngestorError.
#[derive(Debug, Clone, Error)]
#[error("source error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct EngineError {
	pub kind: EngineErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// Generic Result type for source type operations.
pub type EngineResult<T> = Result<T, EngineError>;

impl EngineError {
	pub fn new(kind: EngineErrorKind, source: Arc<anyhow::Error>) -> Self {
		EngineError { kind, source }
	}

	/// Add some context to the wrapper error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		EngineError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the corresponding `IngestorErrorKind` for this error.
	pub fn kind(&self) -> EngineErrorKind {
		self.kind
	}
}

impl From<io::Error> for EngineError {
	fn from(err: io::Error) -> EngineError {
		match err.kind() {
			io::ErrorKind::NotFound =>
				EngineError::new(EngineErrorKind::NotFound, Arc::new(err.into())),
			_ => EngineError::new(EngineErrorKind::Io, Arc::new(err.into())),
		}
	}
}

impl From<serde_json::Error> for EngineError {
	fn from(err: serde_json::Error) -> EngineError {
		EngineError::new(EngineErrorKind::Io, Arc::new(err.into()))
	}
}

impl From<LLMError> for EngineError {
	fn from(err: LLMError) -> EngineError {
		EngineError::new(EngineErrorKind::Io, Arc::new(err.into()))
	}
}

impl From<CandleCoreError> for EngineError {
	fn from(err: CandleCoreError) -> EngineError {
		EngineError::new(EngineErrorKind::ModelError, Arc::new(err.into()))
	}
}

/// Engine trait.
#[async_trait]
pub trait Engine: Send + Sync {
	async fn process_ingested_tokens<'life0>(
		&'life0 self,
		token_stream: Receiver<IngestedTokens>,
	) -> EngineResult<Pin<Box<dyn Stream<Item = EngineResult<EventState>> + Send + 'life0>>>;
}

/// Debugging trait for Engine.
impl std::fmt::Debug for dyn Engine {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Engine")
	}
}
