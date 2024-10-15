use std::{
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};

use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::NotionConfig;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

use super::utils::{
	extract_file_extension, fetch_all_page_ids, fetch_page, format_page, get_images_from_page,
	string_to_async_read,
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NotionSource {
	api_token: String,
	page_ids: Vec<String>,
	source_id: String,
}

impl NotionSource {
	pub async fn new(config: NotionConfig) -> anyhow::Result<Self> {
		Ok(NotionSource {
			api_token: config.api_key.clone(),
			page_ids: config.page_ids.clone(),
			source_id: config.id,
		})
	}
}

#[async_trait]
impl Source for NotionSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let page_ids = if self.page_ids.is_empty() {
			match fetch_all_page_ids(&self.api_token).await {
				Ok(ids) => ids,
				Err(err) =>
					return Err(SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Failed to fetch page IDs: {:?}", err).into(),
					)
					.into()),
			}
		} else {
			self.page_ids.clone()
		};
		let response = fetch_page(&self.api_token, &page_ids[0]).await;

		if response.is_ok() {
			Ok(())
		} else {
			Err(anyhow::anyhow!(
				"Failed to verify API token or page ID, error: {:?}",
				response.err()
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
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let page_ids = self.page_ids.clone();

		let api_token = self.api_token.clone();
		let source_id = self.source_id.clone();

		let stream = stream! {
			let page_ids = if page_ids.is_empty() {
				match fetch_all_page_ids(&api_token).await {
					Ok(ids) => ids,
					Err(err) => {
						yield Err(SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Failed to fetch page IDs: {:?}", err).into(),
						));
						return;
					}
				}
			} else {
				self.page_ids.clone()
			};

			for page_id in page_ids {
				match fetch_page(&api_token, &page_id).await {
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

						let res = get_images_from_page(page.properties.properties).await;
						if let Ok(images) = res {
							for (image_name, image_bytes) in images {
								let image_name_path = Some(PathBuf::from(image_name.clone()));
								let extension = extract_file_extension(&image_name).unwrap_or("").to_string();

								let collected_bytes = CollectedBytes {
									file: image_name_path,
									data: Some(Box::pin(std::io::Cursor::new(image_bytes.clone()))),
									eof: true,
									doc_source: Some("notion://image".to_string()),
									size: Some(image_bytes.len()),
									source_id: source_id.clone(),
									_owned_permit: None,
									image_id: Some(image_name),
									extension: Some(extension),
								};

								yield Ok(collected_bytes);
							}
						}
					},
					Err(err) => {
						yield Err(SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error while fetching page: {:?}", err).into(),
						));
					}
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
			page_ids: vec![],
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
						Ok(collected_bytes) =>
							if let Some(pathbuf) = collected_bytes.file {
								if let Some(str_path) = pathbuf.to_str() {
									count_files.insert(str_path.to_string());
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
			page_ids: vec![env::var("NOTION_PAGE_ID").unwrap_or_default()],
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
			page_ids: vec!["INVALID_PAGE_ID".to_string()],
			id: "NS1".to_string(),
		};

		let notion_api_client = NotionSource::new(notion_config).await.unwrap();

		let connectivity = notion_api_client.check_connectivity().await;
		assert!(connectivity.is_err(), "Connectivity check should pass with a valid API key");
	}
}
