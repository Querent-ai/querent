use std::{
	collections::HashMap,
	io::Cursor,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
	str::FromStr,
};

use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use notion::{
	ids::{DatabaseId, PageId},
	models::{
		properties::{PropertyConfiguration, PropertyValue},
		Properties,
	},
	NotionApi,
};
use proto::semantics::NotionConfig;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, BufReader};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

fn string_to_async_read(description: String) -> impl AsyncRead + Send + Unpin {
	Cursor::new(description.into_bytes())
}

pub fn format_properties(properties: &HashMap<String, PropertyConfiguration>) -> String {
	let mut result = String::new();
	let mut first = true;

	for (key, value) in properties {
		let prop_type = match value {
			PropertyConfiguration::Title { .. } => "Title",
			PropertyConfiguration::Text { .. } => "Text",
			PropertyConfiguration::Number { .. } => "Number",
			PropertyConfiguration::Select { select, id: _ } => &{
				let options = select
					.options
					.iter()
					.map(|opt| format!("{} ({:?})", opt.name, opt.color))
					.collect::<Vec<String>>()
					.join(", ");
				format!("Select [{}]", options)
			},
			PropertyConfiguration::Status { .. } => "Status",
			PropertyConfiguration::MultiSelect { multi_select, id: _ } => &{
				let options = multi_select
					.options
					.iter()
					.map(|opt| format!("{} ({:?})", opt.name, opt.color))
					.collect::<Vec<String>>()
					.join(", ");
				format!("[{}]", options)
			},
			PropertyConfiguration::Date { .. } => "Date",
			PropertyConfiguration::People { .. } => "People",
			PropertyConfiguration::Files { .. } => "Files",
			PropertyConfiguration::Checkbox { .. } => "Checkbox",
			PropertyConfiguration::Url { .. } => "Url",
			PropertyConfiguration::Email { .. } => "Email",
			PropertyConfiguration::PhoneNumber { .. } => "PhoneNumber",
			PropertyConfiguration::Formula { .. } => "Formula",
			PropertyConfiguration::Relation { .. } => "Relation",
			PropertyConfiguration::Rollup { .. } => "Rollup",
			PropertyConfiguration::CreatedTime { .. } => "CreatedTime",
			PropertyConfiguration::CreatedBy { .. } => "CreatedBy",
			PropertyConfiguration::LastEditedTime { .. } => "LastEditedTime",
			PropertyConfiguration::LastEditBy { .. } => "LastEditBy",
			PropertyConfiguration::UniqueId { .. } => "UniqueId",
			PropertyConfiguration::Button { .. } => "Button",
		};

		if first {
			first = false;
		} else {
			result.push_str(", ");
		}

		result.push_str(&format!("{}: {}", key, prop_type));
	}

	result
}

pub fn format_page(properties: &Properties) -> String {
	let mut values: Vec<String> = Vec::new();

	for (key, value) in &properties.properties {
		match value {
			PropertyValue::Title { title, .. } | PropertyValue::Text { rich_text: title, .. } => {
				values.extend(title.iter().map(|t| format!("{}: {}", key, t.plain_text())));
			},
			PropertyValue::Number { number, .. } => {
				if let Some(num) = number {
					values.push(format!("{}: {}", key, num));
				}
			},
			PropertyValue::Select { select, .. } | PropertyValue::Status { status: select, .. } => {
				if let Some(sel) = select {
					values.push(format!("{}: {:?}", key, sel.name));
				}
			},
			PropertyValue::MultiSelect { multi_select, .. } => {
				if let Some(ms) = multi_select {
					values.extend(ms.iter().map(|s| format!("{}: {:?}", key, s.name)));
				}
			},
			PropertyValue::Date { date, .. } => {
				if let Some(d) = date {
					values.push(format!("{}: {:?}", key, d.start));
				}
			},
			PropertyValue::Relation { relation, .. } => {
				if let Some(rel) = relation {
					values.extend(rel.iter().map(|r| format!("{}: {}", key, r.id)));
				}
			},
			PropertyValue::Rollup { rollup, .. } => {
				if let Some(r) = rollup {
					values.push(format!("{}: {}", key, format!("{:?}", r)));
				}
			},
			PropertyValue::Files { files, .. } => {
				if let Some(fs) = files {
					values.extend(fs.iter().map(|f| format!("{}: {}", key, f.name)));
				}
			},
			PropertyValue::Checkbox { checkbox, .. } => {
				values.push(format!("{}: {}", key, checkbox));
			},
			PropertyValue::Url { url, .. } => {
				if let Some(u) = url {
					values.push(format!("{}: {}", key, u));
				}
			},
			PropertyValue::Email { email, .. } => {
				if let Some(e) = email {
					values.push(format!("{}: {}", key, e));
				}
			},
			PropertyValue::PhoneNumber { phone_number, .. } => {
				values.push(format!("{}: {}", key, phone_number));
			},
			PropertyValue::CreatedTime { created_time, .. }
			| PropertyValue::LastEditedTime { last_edited_time: created_time, .. } => {
				values.push(format!("{}: {}", key, created_time));
			},
			PropertyValue::CreatedBy { created_by, .. }
			| PropertyValue::LastEditedBy { last_edited_by: created_by, .. } => {
				values.push(format!("{}: {:?}", key, created_by));
			},
			PropertyValue::UniqueId { unique_id, .. } => {
				values.push(format!("{}: {:?}", key, unique_id));
			},
			PropertyValue::Button { .. } => {
				values.push(format!("{}: Button", key));
			},
			PropertyValue::Formula { id: _, formula } => {
				values.push(format!("{}: {:?}", key, formula));
			},
			PropertyValue::People { id: _, people } => {
				values.push(format!("{}: {:?}", key, people));
			},
		}
	}

	values.join(", ")
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NotionSource {
	api_token: String,
	query_id: String,
	query_type: String,
	source_id: String,
}

impl NotionSource {
	pub async fn new(config: NotionConfig) -> anyhow::Result<Self> {
		Ok(NotionSource {
			api_token: config.api_key.clone(),
			query_id: config.query_id.clone(),
			query_type: config.query_type().as_str_name().to_owned().clone(),
			source_id: config.id,
		})
	}

	pub async fn get_data(&self, notion_api: NotionApi) -> Result<(String, String), SourceError> {
		match self.query_type.as_str() {
			"Database" => {
				let database_id = DatabaseId::from_str(&self.query_id).map_err(|err| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error while creating the database ID object: {:?}", err)
							.into(),
					)
				})?;
				let database = notion_api.get_database(database_id).await.map_err(|err| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error while getting the database: {:?}", err).into(),
					)
				})?;

				let data = format_properties(&database.properties);

				let title_text = database.id;
				let title = format!("{}.notion", title_text);

				Ok((title, data))
			},
			"Page" => {
				let page_id = PageId::from_str(&self.query_id).map_err(|err| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error while creating the page ID object: {:?}", err)
							.into(),
					)
				})?;
				let page = notion_api.get_page(page_id).await.map_err(|err| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error while getting the page: {:?}", err).into(),
					)
				})?;

				let data = format_page(&page.properties);

				let title_text = page.id;
				let title = format!("{}.notion", title_text);

				Ok((title, data))
			},
			_ => {
				return Err(SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Unsupported query_type: {}", self.query_type).into(),
				));
			},
		}
	}
}

#[async_trait]
impl Source for NotionSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _notion_api = NotionApi::new(self.api_token.clone()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while creating Notion API client: {:?}", err).into(),
			)
		})?;
		Ok(())
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		let notion_api = NotionApi::new(self.api_token.clone()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while creating Notion API client: {:?}", err).into(),
			)
		})?;

		let result = self.get_data(notion_api).await;

		let data: String;
		match result {
			Ok((_title, data_str)) => {
				data = data_str;
			},
			Err(e) => {
				return Err(SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Getting error extracting notion data: {}", e).into(),
				));
			},
		}

		Ok(data.into())
	}

	async fn get_slice_stream(
		&self,
		_path: &Path,
		_range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let notion_api = NotionApi::new(self.api_token.clone()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while creating Notion API client: {:?}", err).into(),
			)
		})?;

		let result = self.get_data(notion_api).await;

		let data: String;
		match result {
			Ok((_title, data_str)) => {
				data = data_str;
			},
			Err(e) => {
				return Err(SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Getting error extracting notion data: {}", e).into(),
				));
			},
		}

		Ok(Box::new(string_to_async_read(data)))
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		let notion_api = NotionApi::new(self.api_token.clone()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while creating Notion API client: {:?}", err).into(),
			)
		})?;

		let result = self.get_data(notion_api).await;

		let data: String;
		match result {
			Ok((_title, data_str)) => {
				data = data_str;
			},
			Err(e) => {
				return Err(SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Getting error extracting notion data: {}", e).into(),
				));
			},
		}

		Ok(data.into())
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		let notion_api = NotionApi::new(self.api_token.clone()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while creating Notion API client: {:?}", err).into(),
			)
		})?;

		let result = self.get_data(notion_api).await;

		let data: String;
		match result {
			Ok((_title, data_str)) => {
				data = data_str;
			},
			Err(e) => {
				return Err(SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Getting error extracting notion data: {}", e).into(),
				));
			},
		}

		Ok(data.len() as u64)
	}

	async fn copy_to(&self, _path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let notion_api = NotionApi::new(self.api_token.clone()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while creating Notion API client: {:?}", err).into(),
			)
		})?;

		let result = self.get_data(notion_api).await;

		let data: String;
		match result {
			Ok((_title, data_str)) => {
				data = data_str;
			},
			Err(e) => {
				return Err(SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Getting error extracting notion data: {}", e).into(),
				));
			},
		}

		let mut body_stream_reader = BufReader::new(data.as_bytes());
		tokio::io::copy_buf(&mut body_stream_reader, output).await?;
		Ok(())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let notion_api = NotionApi::new(self.api_token.clone()).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while creating Notion API client: {:?}", err).into(),
			)
		})?;
		let source_id = self.source_id.clone();

		let file_name_path: Option<PathBuf>;
		let data: String;

		let result = self.get_data(notion_api).await;
		match result {
			Ok((title, data_str)) => {
				data = data_str;
				file_name_path = Some(PathBuf::from(title));
			},
			Err(e) => {
				return Err(SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Getting error extracting notion data: {}", e).into(),
				));
			},
		}

		let doc_source = Some("notion://".to_string());
		let data_len = data.len();
		let collected_bytes = CollectedBytes::new(
			file_name_path,
			Some(Box::pin(string_to_async_read(data))),
			true,
			doc_source,
			Some(data_len),
			source_id.clone(),
			None,
		);

		let stream = stream! {
			yield Ok(collected_bytes);
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
			query_type: 0,
			query_id: env::var("NOTION_PAGE_ID")
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
						Err(_) => panic!("Expected successful data collection"),
					}
				}
				println!("Files are --- {:?}", count_files);
			},
			Err(e) => {
				eprintln!("Failed to get stream: {:?}", e);
			},
		}
	}
}
