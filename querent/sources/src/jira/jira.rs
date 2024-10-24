use std::{
	io::Cursor,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};

use async_stream::stream;
use async_trait::async_trait;
use bytes::Bytes;
use common::{retry, CollectedBytes};
use futures::Stream;
use jira_query::Issue;
use proto::semantics::JiraCollectorConfig;
use reqwest::Client;
use tokio::io::AsyncRead;

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

#[derive(Debug, Clone)]
pub struct AttachmentJira {
	pub file_name: String,
	pub data: Bytes,
}

#[derive(Clone, Debug)]
pub struct JiraSource {
	jira_server: String,
	jira_email: String,
	jira_api_token: String,
	project_id: String,
	source_id: String,
	pub retry_params: common::RetryParams,
}

impl JiraSource {
	pub async fn new(config: JiraCollectorConfig) -> anyhow::Result<Self> {
		Ok(JiraSource {
			jira_api_token: config.jira_api_key,
			jira_email: config.jira_email,
			jira_server: config.jira_server,
			project_id: config.jira_project,
			source_id: config.id,
			retry_params: common::RetryParams::aggressive(),
		})
	}
}

pub async fn get_issues(
	client: &Client,
	jira_server: String,
	email: String,
	api_token: String,
	jql_query: String,
	start_at: u32,
	max_results: u32,
	retry_params: &common::RetryParams,
) -> Result<(Vec<Issue>, Vec<AttachmentJira>), SourceError> {
	let jira_url = format!("{}/rest/api/2/search", jira_server.clone());
	let response = retry(retry_params, || async {
		client
			.get(jira_url.clone())
			.basic_auth(email.clone(), Some(api_token.clone()))
			.query(&[
				("jql", jql_query.clone()),
				("startAt", start_at.clone().to_string()),
				("maxResults", max_results.clone().to_string()),
			])
			.send()
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error making the API request: {:?}", err).into(),
				)
			})
	})
	.await?;

	if !response.status().is_success() {
		return Err(SourceError::new(
			SourceErrorKind::Io,
			anyhow::anyhow!(
				"Error making the API request: {:?}",
				response.text().await.unwrap_or("Unknown Error".to_string())
			)
			.into(),
		));
	}

	let mut attachments_res: Vec<AttachmentJira> = Vec::new();
	let json_response = response.json::<serde_json::Value>().await.map_err(|e| {
		SourceError::new(
			SourceErrorKind::Io,
			anyhow::anyhow!("Got error while transferring data: {:?}", e).into(),
		)
	})?;

	let issues = json_response
		.get("issues")
		.and_then(|v| v.as_array())
		.ok_or_else(|| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!(
					"Invalid response structure: 'issues' field missing or not an array"
				)
				.into(),
			)
		})?
		.iter()
		.filter_map(|issue| serde_json::from_value(issue.clone()).ok())
		.collect::<Vec<Issue>>();

	for issue in &issues {
		// https://querent.atlassian.net/rest/api/2/search
		let issue_details_url =
			format!("{}/rest/api/2/issue/{}", jira_server.clone(), issue.key.clone());
		let issue_details_response = client
			.get(&issue_details_url)
			.basic_auth(email.clone(), Some(api_token.clone()))
			.send()
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error fetching issue details: {:?}", err).into(),
				)
			})?;

		if !issue_details_response.status().is_success() {
			continue;
		}

		let issue_details =
			issue_details_response.json::<serde_json::Value>().await.map_err(|e| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error parsing issue details: {:?}", e).into(),
				)
			})?;

		if let Some(fields) = issue_details.get("fields") {
			if let Some(attachments) = fields.get("attachment").and_then(|v| v.as_array()) {
				for attachment in attachments {
					if let (Some(content_url), Some(filename)) = (
						attachment.get("content").and_then(|v| v.as_str()),
						attachment.get("filename").and_then(|v| v.as_str()),
					) {
						let attachment_response = client
							.get(content_url)
							.basic_auth(email.clone(), Some(api_token.clone()))
							.send()
							.await
							.map_err(|err| {
								SourceError::new(
									SourceErrorKind::Io,
									anyhow::anyhow!("Error downloading attachment: {:?}", err)
										.into(),
								)
							})?;

						if attachment_response.status().is_success() {
							let bytes = attachment_response.bytes().await.map_err(|err| {
								SourceError::new(
									SourceErrorKind::Io,
									anyhow::anyhow!("Error reading attachment bytes: {:?}", err)
										.into(),
								)
							})?;
							let attachment_data =
								AttachmentJira { file_name: filename.to_string(), data: bytes };
							attachments_res.push(attachment_data);
						}
					}
				}
			}
		}
	}

	if issues.len() > 0 {
		Ok((issues.clone(), attachments_res))
	} else {
		Ok((Vec::new(), Vec::new()))
	}
}

fn string_to_async_read(data: String) -> impl AsyncRead + Send + Unpin {
	Cursor::new(data.into_bytes())
}

#[async_trait]
impl Source for JiraSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let jira_url = format!("{}/rest/api/2/search", self.jira_server.clone());
		let email = self.jira_email.clone();
		let api_token = self.jira_api_token.clone();

		let jql_query = format!("project={}", self.project_id.replace("\"", ""));

		let client = Client::new();

		let response = client
			.get(jira_url)
			.basic_auth(email, Some(api_token))
			.query(&[("jql", jql_query), ("maxResults", "1".to_string())])
			.send()
			.await?;

		if response.status().as_str() != "200" {
			let error_message = response.text().await.unwrap_or("Unknown error".to_string());
			return Err(anyhow::anyhow!("Failed to authenticate with Jira API: {}", error_message));
		}

		Ok(())
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
		let source_id = self.source_id.clone();

		let jira_url = self.jira_server.clone();
		let email = self.jira_email.clone();
		let api_token = self.jira_api_token.clone();

		let jql_query = format!("project={}", self.project_id.replace("\"", ""));

		let client = Client::new();

		let stream = stream! {
			let mut start_at = 0;
			let max_results = 50;
			loop {
				match get_issues(&client, jira_url.clone(), email.clone(), api_token.clone(), jql_query.clone(), start_at, max_results, &self.retry_params).await {
					Ok((issues, attachments)) => {
						if issues.is_empty() {
							break;
						}

						for issue in issues {
							let description = issue.fields.description.clone().unwrap_or("".to_string());
							let summary = issue.fields.summary.clone();
							let issue_str = format!("{}. {}", description, summary);

							let file_name = format!("{}.jira", issue.id.clone());
							let file_name_path = Some(PathBuf::from(file_name));
							let doc_source = Some("jira://".to_string());

							let collected_bytes = CollectedBytes::new(
								file_name_path,
								Some(Box::pin(string_to_async_read(issue_str))),
								true,
								doc_source,
								Some(1),
								source_id.clone(),
								None,
							);
							yield Ok(collected_bytes);
						}

						for attachment in attachments {
							let file_name_path = Some(PathBuf::from(attachment.file_name.clone()));

							let cursor = Cursor::new(attachment.data.to_vec());

							let doc_source = Some("jira://".to_string());
							let collected_bytes = CollectedBytes::new(
								file_name_path,
								Some(Box::pin(cursor)),
								true,
								doc_source,
								Some(1),
								source_id.clone(),
								None,
							);
							yield Ok(collected_bytes);
						}

						start_at += max_results;
					}
					Err(e) => {
						yield Err(e);
						break;
					}
				}
			}

		};

		Ok(Box::pin(stream))
	}
}

#[cfg(test)]
mod tests {
	use std::env;

	use super::*;
	use dotenv::dotenv;
	use futures::StreamExt;

	#[tokio::test]
	async fn test_jira_source() {
		dotenv().ok();

		let jira_config: JiraCollectorConfig = JiraCollectorConfig {
			jira_server: "https://querent.atlassian.net".to_string(),
			jira_email: "ansh@querent.xyz".to_string(),
			jira_api_key: env::var("JIRA_API_TOKEN").unwrap_or("".to_string()),
			jira_project: "SCRUM".to_string(),
			id: "Jira-source".to_string(),
		};

		let jira_source = JiraSource::new(jira_config).await.unwrap();
		let result = jira_source.poll_data().await;

		let mut found_data = false;
		match result {
			Ok(mut stream) =>
				while let Some(item) = stream.next().await {
					if item.is_ok() {
						found_data = true;
					} else if item.is_err() {
						eprintln!("Found error {:?}", item.err());
						break;
					}
				},
			Err(e) => {
				println!("Error is {:?}", e);
				assert!(false, "Expected a stream but encountered an error during stream creation")
			},
		}
		assert!(found_data, "No data found");
	}

	#[tokio::test]
	async fn test_jira_source_with_invalid_key() {
		dotenv().ok();

		let jira_config: JiraCollectorConfig = JiraCollectorConfig {
			jira_server: "https://querent.atlassian.net".to_string(),
			jira_email: "ansh@querent.xyz".to_string(),
			jira_api_key: "Invalid_key".to_string(),
			jira_project: "SCRUM".to_string(),
			id: "Jira-source".to_string(),
		};

		let jira_source = JiraSource::new(jira_config).await.unwrap();

		let connectivity = jira_source.check_connectivity().await;
		assert!(connectivity.is_err(), "Expected an error but got none");
	}

	#[tokio::test]
	async fn test_jira_source_with_invalid_project() {
		dotenv().ok();

		let jira_config: JiraCollectorConfig = JiraCollectorConfig {
			jira_server: "https://querent.atlassian.net".to_string(),
			jira_email: "ansh@querent.xyz".to_string(),
			jira_api_key: env::var("JIRA_API_TOKEN").unwrap_or("".to_string()),
			jira_project: "Invalid project".to_string(),
			id: "Jira-source".to_string(),
		};

		let jira_source = JiraSource::new(jira_config).await.unwrap();

		let connectivity = jira_source.check_connectivity().await;
		assert!(connectivity.is_err(), "Expected an error but got none");
	}
}
