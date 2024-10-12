use std::{
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
	str::FromStr,
};

use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use notion::ids::PageId;
use proto::semantics::NotionConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

use super::utils::{
	fetch_databases_in_page, fetch_page, format_page, format_properties, string_to_async_read,
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NotionSource {
	api_token: String,
	page_id: String,
	source_id: String,
}

impl NotionSource {
	pub async fn new(config: NotionConfig) -> anyhow::Result<Self> {
		Ok(NotionSource {
			api_token: config.api_key.clone(),
			page_id: config.page_id.clone(),
			source_id: config.id,
		})
	}
}

#[async_trait]
impl Source for NotionSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let client = Client::new();
		let page_id = &self.page_id;
		let url = format!("https://api.notion.com/v1/pages/{}", page_id);

		let response = client
			.get(&url)
			.bearer_auth(&self.api_token)
			.header("Notion-Version", "2022-06-28")
			.send()
			.await
			.map_err(|err| {
				anyhow::anyhow!("Error while making request to Notion API: {:?}", err)
			})?;

		if response.status().is_success() {
			Ok(())
		} else {
			Err(anyhow::anyhow!(
				"Failed to verify API token or page ID, status code: {}",
				response.status()
			))
		}
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		Ok(vec![])
	}

	async fn get_slice_stream(
		&self,
		_path: &Path,
		_range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		Ok(Box::new(string_to_async_read("".to_string())))
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		Ok(vec![])
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		Ok(0)
	}

	async fn copy_to(&self, _path: &Path, _output: &mut dyn SendableAsync) -> SourceResult<()> {
		Ok(())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let page_id = PageId::from_str(&self.page_id).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Invalid Page ID: {:?}", err).into(),
			)
		})?;

		let api_token = self.api_token.clone();
		let source_id = self.source_id.clone();

		let stream = stream! {
			match fetch_page(&api_token, &page_id.to_string()).await {
				Ok(page) => {
					let page_data = format_page(&page.properties);
					let page_file_name = format!("{}.notion", page_id);

					yield Ok(CollectedBytes::new(
						Some(PathBuf::from(page_file_name)),
						Some(Box::pin(string_to_async_read(page_data.clone()))),
						true,
						Some("notion://page".to_string()),
						Some(page_data.len()),
						source_id.clone(),
						None,
					));
				},
				Err(err) => {
					yield Err(SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error while fetching page: {:?}", err).into(),
					));
				}
			}

			match fetch_databases_in_page(&api_token, &page_id.to_string()).await {
				Ok(databases) => {
					for database in databases {
						let data = format_properties(&database.properties);
						let title_text = database.id;
						let title = format!("{}.notion", title_text);

						yield Ok(CollectedBytes::new(
							Some(PathBuf::from(title)),
							Some(Box::pin(string_to_async_read(data.clone()))),
							true,
							Some("notion://database".to_string()),
							Some(data.len()),
							source_id.clone(),
							None,
						));
					}
				},
				Err(err) => {
					yield Err(SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error while fetching databases: {:?}", err).into(),
					));
				}
			}
		};

		Ok(Box::pin(stream))
	}
}

#[cfg(test)]
mod tests {

	use std::{collections::HashSet, env};

	use dotenv::dotenv;
	use futures::StreamExt;

	use super::*;

	#[tokio::test]
	async fn test_notion_collector() {
		dotenv().ok();
		let notion_config = NotionConfig {
			api_key: env::var("NOTION_API_KEY")
				.map_err(|e| e.to_string())
				.unwrap_or("".to_string()),
			page_id: env::var("NOTION_PAGE_ID")
				.map_err(|e| e.to_string())
				.unwrap_or("".to_string()),
			id: "NS1".to_string(),
		};

		let notion_api_client = NotionSource::new(notion_config).await.unwrap();

		let connectivity = notion_api_client.check_connectivity().await;
		assert!(
			connectivity.is_ok(),
			"Failed to connect to notion API {:?}",
			connectivity.err().unwrap()
		);

		let result = notion_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut count_files: HashSet<String> = HashSet::new();
				while let Some(item) = stream.next().await {
					match item {
						Ok(collected_bytes) => {
							if let Some(pathbuf) = collected_bytes.file {
								if let Some(str_path) = pathbuf.to_str() {
									count_files.insert(str_path.to_string());
								}
							}
						},
						Err(e) => {
							println!("Error {:?}", e);
							panic!("Expected successful data collection")
						},
					}
				}
				println!("Files are --- {:?}", count_files);
				assert!(count_files.len() > 0, "No files found");
			},
			Err(e) => {
				eprintln!("Failed to get stream: {:?}", e);
			},
		}
	}

	#[tokio::test]
	async fn test_invalid_api_key() {
		dotenv().ok();
		let notion_config = NotionConfig {
			api_key: "INVALID_API_KEY".to_string(),
			page_id: env::var("NOTION_PAGE_ID").unwrap_or_default(),
			id: "NS1".to_string(),
		};

		let notion_api_client = NotionSource::new(notion_config).await.unwrap();

		let connectivity = notion_api_client.check_connectivity().await;
		assert!(
			connectivity.is_err(),
			"Expected connectivity check to fail with an invalid API key"
		);
	}

	#[tokio::test]
	async fn test_invalid_page_id() {
		dotenv().ok();
		let notion_config = NotionConfig {
			api_key: env::var("NOTION_API_KEY").unwrap_or_default(),
			page_id: "INVALID_PAGE_ID".to_string(),
			id: "NS1".to_string(),
		};

		let notion_api_client = NotionSource::new(notion_config).await.unwrap();

		let connectivity = notion_api_client.check_connectivity().await;
		assert!(connectivity.is_err(), "Connectivity check should pass with a valid API key");
	}
}
