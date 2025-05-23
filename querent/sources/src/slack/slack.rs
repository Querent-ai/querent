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

use std::{
	io::Cursor,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};

use async_stream::stream;
use async_trait::async_trait;
use common::{retry, CollectedBytes};
use futures::Stream;
use proto::semantics::SlackCollectorConfig;
use slack_morphism::{
	api::{SlackApiConversationsHistoryRequest, SlackApiConversationsHistoryResponse},
	prelude::SlackClientHyperConnector,
	SlackApiToken, SlackApiTokenValue, SlackChannelId, SlackClient, SlackCursorId, SlackTs,
};
use tokio::io::AsyncRead;

use crate::{DataSource, SendableAsync, SourceError, SourceErrorKind, SourceResult};

#[derive(Clone, Debug)]
pub struct SlackApiClient {
	source_id: String,
	token: SlackApiToken,
	pub channel: String,
	latest: Option<String>,
	inclusive: Option<bool>,
	oldest: Option<String>,
	pub retry_params: common::RetryParams,
}

impl SlackApiClient {
	pub async fn new(config: SlackCollectorConfig) -> anyhow::Result<Self> {
		let token_value: SlackApiTokenValue = config.access_token.clone().into();
		let token: SlackApiToken = SlackApiToken::new(token_value.clone());

		Ok(SlackApiClient {
			source_id: config.id,
			token,
			channel: config.channel_name,
			latest: config.latest,
			inclusive: config.inclusive,
			oldest: config.oldest,
			retry_params: common::RetryParams::aggressive(),
		})
	}
}

fn string_to_async_read(description: String) -> impl AsyncRead + Send + Unpin {
	Cursor::new(description.into_bytes())
}

fn remove_special_characters(input: &str) -> String {
	input.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).collect()
}

pub async fn get_message(
	req: SlackApiConversationsHistoryRequest,
	token: &SlackApiToken,
) -> Result<SlackApiConversationsHistoryResponse, SourceError> {
	let client = SlackClient::new(SlackClientHyperConnector::new()?);
	let session = client.open_session(token);

	let message_response = session.conversations_history(&req).await.map_err(|err| {
		SourceError::new(
			SourceErrorKind::Io,
			anyhow::anyhow!("Error while getting the messages: {:?}", err).into(),
		)
	})?;

	Ok(message_response)
}

#[async_trait]
impl DataSource for SlackApiClient {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let client = SlackClient::new(SlackClientHyperConnector::new()?);

		let session = client.open_session(&self.token);

		match session.auth_test().await {
			Ok(_) => {},
			Err(e) => {
				return Err(anyhow::anyhow!(
					"Failed to make the connection to slack source {:?}",
					e
				));
			},
		};

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
		let channel_id: SlackChannelId = SlackChannelId::new(self.channel.clone());
		let latest: SlackTs = SlackTs::new(self.latest.clone().unwrap_or("".to_string()));
		let oldest: SlackTs = SlackTs::new(self.oldest.clone().unwrap_or("".to_string()));
		let token = self.token.clone();
		let limit = 100;

		let source_id = self.source_id.clone();
		let mut cursor: Option<SlackCursorId> = None;
		let stream = stream! {
			loop {
				let req: SlackApiConversationsHistoryRequest = SlackApiConversationsHistoryRequest {
					channel: Some(channel_id.clone()),
					cursor: cursor.clone(),
					latest: Some(latest.clone()),
					limit: Some(limit.clone()),
					oldest: Some(oldest.clone()),
					inclusive: Some(self.inclusive.unwrap_or(true)),
				};
				let message_response = retry(&self.retry_params, || async {
					get_message(req.clone(), &token)
						.await
				})
				.await?;
				for messages in message_response.messages {
					let file_name = format!("{}.slack", channel_id.to_string());
					let file_name_path = Some(PathBuf::from(file_name));
					let mut message_text = messages.content.text.unwrap_or("".to_string());
					if let Some(attachments) = messages.content.attachments {
						if !attachments.is_empty() {
							// finding links in the attachments
							for attachment in attachments {
								if let Some(mut fallback) = attachment.fallback {
									fallback = remove_special_characters(&fallback);
									message_text.push_str(" ");
									message_text.push_str(&fallback);
								}
								if let Some(text) = attachment.text {
									message_text.push_str(" ");
									message_text.push_str(&text);
								}
							}
						}
					} else {
						continue;
					}
					let doc_source = Some("slack://".to_string());
					let data_len = Some(message_text.len());
					yield Ok(CollectedBytes::new(
						file_name_path,
						Some(Box::pin(string_to_async_read(message_text))),
						true,
						doc_source,
						data_len,
						source_id.clone(),
						None
					))
				}
				if let Some(metadata) = message_response.response_metadata {
					cursor = metadata.next_cursor;
				} else {
					cursor = None;
				}
				if cursor.is_none() {
					break;
				}
				if !message_response.has_more.unwrap_or(false) {
					break;
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
	async fn test_slack_collector() {
		dotenv().ok();

		let _ = rustls::crypto::ring::default_provider().install_default();

		let slack_config = SlackCollectorConfig {
			access_token: env::var("SLACK_API_TOKEN")
				.map_err(|e| e.to_string())
				.unwrap_or("".to_string()),
			channel_name: env::var("SLACK_CHANNEL_NAME")
				.map_err(|e| e.to_string())
				.unwrap_or("".to_string()),
			oldest: None,
			latest: None,
			inclusive: Some(true),
			id: "Slack-source".to_string(),
		};

		let slack_api_client = SlackApiClient::new(slack_config).await.unwrap();

		let connectivity = slack_api_client.check_connectivity().await;
		assert!(
			connectivity.is_ok(),
			"Failed to connect to slack API {:?}",
			connectivity.err().unwrap()
		);

		let result = slack_api_client.poll_data().await;

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

	#[tokio::test]
	async fn test_slack_collector_invalid_token() {
		dotenv().ok();

		let _ = rustls::crypto::ring::default_provider().install_default();

		let slack_config = SlackCollectorConfig {
			access_token: "invalid_token".to_string(),
			channel_name: env::var("SLACK_CHANNEL_NAME")
				.map_err(|e| e.to_string())
				.unwrap_or("".to_string()),
			oldest: None,
			latest: None,
			inclusive: Some(true),
			id: "Slack-source".to_string(),
		};

		let slack_api_client = SlackApiClient::new(slack_config)
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while creating slack api client: {:?}", err).into(),
				)
			})
			.unwrap();
		let response = slack_api_client.check_connectivity().await;

		assert!(response.is_err(), "Expected authentication error, got {:?}", response.err());
	}

	#[tokio::test]
	async fn test_slack_collector_invalid_channel() {
		dotenv().ok();

		let _ = rustls::crypto::ring::default_provider().install_default();

		let slack_config = SlackCollectorConfig {
			access_token: env::var("SLACK_API_TOKEN")
				.map_err(|e| e.to_string())
				.unwrap_or("".to_string()),
			channel_name: "invalid_channel".to_string(),
			oldest: None,
			latest: None,
			inclusive: Some(true),
			id: "Slack-source".to_string(),
		};

		let slack_api_client = SlackApiClient::new(slack_config)
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while creating slack api client: {:?}", err).into(),
				)
			})
			.unwrap();
		let response = slack_api_client.check_connectivity().await;

		assert!(!response.is_err(), "Got authentication error{:?}", response.err());

		let result = slack_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut found_error = false;
				while let Some(item) = stream.next().await {
					match item {
						Ok(_collected_bytes) => {},
						Err(_) => found_error = true,
					}
				}
				assert!(found_error, "No error found");
			},
			Err(e) => {
				eprintln!("Failed to get stream: {:?}", e);
			},
		}
	}

	#[tokio::test]
	async fn test_slack_collector_channel_not_joined() {
		dotenv().ok();

		let _ = rustls::crypto::ring::default_provider().install_default();

		let slack_config = SlackCollectorConfig {
			access_token: env::var("SLACK_API_TOKEN")
				.map_err(|e| e.to_string())
				.unwrap_or("".to_string()),
			channel_name: "C06KGBF8A7N".to_string(),
			oldest: None,
			latest: None,
			inclusive: Some(true),
			id: "Slack-source".to_string(),
		};

		let slack_api_client = SlackApiClient::new(slack_config)
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while creating slack api client: {:?}", err).into(),
				)
			})
			.unwrap();
		let response = slack_api_client.check_connectivity().await;

		assert!(!response.is_err(), "Got authentication error{:?}", response.err());

		let result = slack_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut found_error = false;
				while let Some(item) = stream.next().await {
					match item {
						Ok(_collected_bytes) => {},
						Err(_) => found_error = true,
					}
				}
				assert!(found_error, "No error found");
			},
			Err(e) => {
				eprintln!("Failed to get stream: {:?}", e);
			},
		}
	}
}
