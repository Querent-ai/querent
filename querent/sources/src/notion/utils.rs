use std::{collections::HashMap, io::Cursor, path::PathBuf};

use chrono::{DateTime, Utc};
use notion::{
	ids::PageId,
	models::{
		properties::{PropertyConfiguration, PropertyValue},
		Database, Properties,
	},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncRead;

use crate::{SourceError, SourceErrorKind};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Page {
	pub object: String,
	pub id: PageId,
	pub created_time: DateTime<Utc>,
	pub last_edited_time: DateTime<Utc>,
	pub archived: bool,
	pub icon: Option<serde_json::Value>,
	pub cover: Option<serde_json::Value>,
	pub properties: Properties,
	pub parent: serde_json::Value,
	pub url: String,
}

pub fn string_to_async_read(description: String) -> impl AsyncRead + Send + Unpin {
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
			PropertyValue::Number { number, .. } =>
				if let Some(num) = number {
					values.push(format!("{}: {}", key, num));
				},
			PropertyValue::Select { select, .. } | PropertyValue::Status { status: select, .. } =>
				if let Some(sel) = select {
					values.push(format!("{}: {:?}", key, sel.name));
				},
			PropertyValue::MultiSelect { multi_select, .. } =>
				if let Some(ms) = multi_select {
					values.extend(ms.iter().map(|s| format!("{}: {:?}", key, s.name)));
				},
			PropertyValue::Date { date, .. } =>
				if let Some(d) = date {
					values.push(format!("{}: {:?}", key, d.start));
				},
			PropertyValue::Relation { relation, .. } =>
				if let Some(rel) = relation {
					values.extend(rel.iter().map(|r| format!("{}: {}", key, r.id)));
				},
			PropertyValue::Rollup { rollup, .. } =>
				if let Some(r) = rollup {
					values.push(format!("{}: {}", key, format!("{:?}", r)));
				},
			PropertyValue::Files { files, .. } =>
				if let Some(fs) = files {
					values.extend(fs.iter().map(|f| format!("{}: {}", key, f.name)));
				},
			PropertyValue::Checkbox { checkbox, .. } => {
				values.push(format!("{}: {}", key, checkbox));
			},
			PropertyValue::Url { url, .. } =>
				if let Some(u) = url {
					values.push(format!("{}: {}", key, u));
				},
			PropertyValue::Email { email, .. } =>
				if let Some(e) = email {
					values.push(format!("{}: {}", key, e));
				},
			PropertyValue::PhoneNumber { phone_number, .. } => {
				values.push(format!("{}: {}", key, phone_number));
			},
			PropertyValue::CreatedTime { created_time, .. } |
			PropertyValue::LastEditedTime { last_edited_time: created_time, .. } => {
				values.push(format!("{}: {}", key, created_time));
			},
			PropertyValue::CreatedBy { created_by, .. } |
			PropertyValue::LastEditedBy { last_edited_by: created_by, .. } => {
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

pub async fn fetch_page(api_token: &str, page_id: &str) -> Result<Page, SourceError> {
	let client = Client::new();
	let url = format!("https://api.notion.com/v1/pages/{}", page_id);

	let response = client
		.get(&url)
		.bearer_auth(api_token)
		.header("Notion-Version", "2022-06-28")
		.send()
		.await
		.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while making request to Notion API: {:?}", err).into(),
			)
		})?;

	if response.status().is_success() {
		let page: Page = response.json::<Page>().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error parsing Notion page response: {:?}", err).into(),
			)
		})?;
		Ok(page)
	} else {
		Err(SourceError::new(
			SourceErrorKind::Io,
			anyhow::anyhow!("Failed to fetch page from Notion API").into(),
		))
	}
}

pub fn remove_hyphens(s: &str) -> String {
	s.replace("-", "")
}

pub async fn fetch_databases_in_page(
	api_token: &str,
	page_id: &str,
) -> Result<Vec<Database>, SourceError> {
	let client = Client::new();
	let url = "https://api.notion.com/v1/search";

	let body = json!({
		"filter": {
			"property": "object",
			"value": "database"
		}
	});

	let response = client
		.post(url)
		.bearer_auth(api_token)
		.header("Notion-Version", "2022-06-28")
		.json(&body)
		.send()
		.await
		.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while making request to Notion API: {:?}", err).into(),
			)
		})?;

	if response.status().is_success() {
		let list_response: serde_json::Value =
			response.json::<serde_json::Value>().await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error parsing Notion databases response: {:?}", err).into(),
				)
			})?;

		let databases = list_response["results"]
			.as_array()
			.unwrap_or(&vec![])
			.iter()
			.filter(|db| {
				if let Some(parent) = db.get("parent") {
					if let Some(parent_id) = parent.get("page_id") {
						let clean_parent_id = remove_hyphens(parent_id.as_str().unwrap_or(""));
						return clean_parent_id == page_id;
					}
				}
				false
			})
			.map(|db| serde_json::from_value::<Database>(db.clone()))
			.collect::<Result<Vec<_>, _>>()
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error parsing database object: {:?}", err).into(),
				)
			})?;

		Ok(databases)
	} else {
		Err(SourceError::new(
			SourceErrorKind::Io,
			anyhow::anyhow!("Failed to fetch databases from Notion API").into(),
		))
	}
}

async fn get_image(client: &reqwest::Client, url: &str) -> Result<(String, Vec<u8>), SourceError> {
	let response = client
		.get(url)
		.header(reqwest::header::USER_AGENT, "Querent/1.0")
		.send()
		.await
		.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error making the API request: {:?}", err).into(),
			)
		})?;

	if !response.status().is_success() {
		return Err(SourceError::new(
			SourceErrorKind::Io,
			anyhow::anyhow!("Failed to download image: HTTP status {}", response.status()).into(),
		));
	}

	let image_bytes = response.bytes().await.map_err(|err| {
		SourceError::new(
			SourceErrorKind::Io,
			anyhow::anyhow!("Error reading response body: {:?}", err).into(),
		)
	})?;

	let image_name = extract_image_name(url)?;

	Ok((image_name, image_bytes.to_vec()))
}

fn extract_image_name(url: &str) -> Result<String, SourceError> {
	let path = url.split('/').last().ok_or_else(|| {
		SourceError::new(
			SourceErrorKind::Io,
			anyhow::anyhow!("Failed to extract image name from URL").into(),
		)
	})?;

	let binding = PathBuf::from(path);
	let file_name = binding.file_name().and_then(|name| name.to_str()).ok_or_else(|| {
		SourceError::new(SourceErrorKind::Io, anyhow::anyhow!("Invalid image name in URL").into())
	})?;

	Ok(file_name.to_string())
}

pub async fn get_images_from_page(
	properties: HashMap<String, PropertyValue>,
) -> Result<Vec<(String, Vec<u8>)>, SourceError> {
	let mut images = Vec::new();
	let client = Client::new();

	for (_key, value) in properties {
		match value {
			PropertyValue::Files { files, .. } =>
				if let Some(file_list) = files {
					for file in file_list {
						match get_image(&client, &file.name).await {
							Ok((image_name, image_bytes)) => {
								images.push((image_name, image_bytes));
							},
							Err(err) => {
								return Err(SourceError::new(
									SourceErrorKind::Io,
									anyhow::anyhow!("Failed to fetch image: {:?}", err).into(),
								));
							},
						}
					}
				},
			_ => {},
		}
	}

	Ok(images)
}

pub fn extract_file_extension(file_name: &str) -> Option<&str> {
	file_name.split('.').last()
}
