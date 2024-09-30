use std::{io::Cursor, ops::Range, path::Path, pin::Pin};

use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;

use crate::{SendableAsync, Source, SourceResult};

#[derive(Deserialize, Serialize, Clone, Debug)]
struct NotionSource {
	api_token: Option<String>,
	task_database_id: Option<String>,
}

impl NotionSource {
	pub async fn new(api_key: String, database_id: String) -> anyhow::Result<Self> {
		Ok(NotionSource {
			api_token: Some(api_key.clone()),
			task_database_id: Some(database_id.clone()),
		})
	}
}

fn string_to_async_read(description: String) -> impl AsyncRead + Send + Unpin {
	Cursor::new(description.into_bytes())
}

#[async_trait]
impl Source for NotionSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		Ok(())
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		Ok(Vec::new())
	}

	async fn get_slice_stream(
		&self,
		_path: &Path,
		_range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		Ok(Box::new(string_to_async_read("".to_string())))
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		Ok(Vec::new())
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		Ok(34 as u64)
	}

	async fn copy_to(&self, _path: &Path, _output: &mut dyn SendableAsync) -> SourceResult<()> {
		Ok(())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
	}
}
