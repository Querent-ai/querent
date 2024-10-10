use std::{
	collections::VecDeque,
	io::Cursor,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};

use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use proto::semantics::SlackCollectorConfig;
use slack_morphism::{
	api::SlackApiConversationsHistoryRequest, prelude::SlackClientHyperConnector, SlackApiToken,
	SlackApiTokenValue, SlackChannelId, SlackClient, SlackClientMessageId, SlackCursorId, SlackTs,
};
use tokio::io::{AsyncRead, BufReader};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

#[derive(Clone, Debug)]
pub struct SlackApiClient {
	client: SlackClient<SlackClientHyperConnector<HttpsConnector<HttpConnector>>>,
	source_id: String,
	token: SlackApiToken,
	pub channel: String,
	cursor: Option<String>,
	latest: Option<String>,
	inclusive: Option<bool>,
	limit: Option<u16>,
	oldest: Option<String>,
}

impl SlackApiClient {
	pub async fn new(config: SlackCollectorConfig) -> anyhow::Result<Self> {
		let client = SlackClient::new(SlackClientHyperConnector::new()?);
		let token_value: SlackApiTokenValue = config.access_token.clone().into();
		let token: SlackApiToken = SlackApiToken::new(token_value.clone());

		Ok(SlackApiClient {
			client,
			source_id: config.id,
			token,
			channel: config.channel_name,
			cursor: config.cursor,
			latest: config.latest,
			inclusive: config.inclusive,
			limit: config.limit.map(|l| l as u16),
			oldest: config.oldest,
		})
	}
}

fn string_to_async_read(description: String) -> impl AsyncRead + Send + Unpin {
	Cursor::new(description.into_bytes())
}

#[async_trait]
impl Source for SlackApiClient {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		Ok(())
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		let session = self.client.open_session(&self.token);
		let channel_id: SlackChannelId = SlackChannelId::new(self.channel.clone());
		let mut cursor: Option<SlackCursorId> =
			Some(SlackCursorId::new(self.cursor.clone().unwrap_or("".to_string())));
		let latest: SlackTs = SlackTs::new(self.latest.clone().unwrap_or("".to_string()));
		let oldest: SlackTs = SlackTs::new(self.oldest.clone().unwrap_or("".to_string()));
		let mut all_messages = String::new();

		loop {
			let req: SlackApiConversationsHistoryRequest = SlackApiConversationsHistoryRequest {
				channel: Some(channel_id.clone()),
				cursor: cursor.clone(),
				latest: Some(latest.clone()),
				limit: Some(self.limit.unwrap_or(100 as u16)),
				oldest: Some(oldest.clone()),
				inclusive: Some(self.inclusive.unwrap_or(true)),
			};
			let message_response = session.conversations_history(&req).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while getting the messages: {:?}", err).into(),
				)
			})?;

			for messages in message_response.messages {
				all_messages.push_str(&messages.content.text.unwrap_or("".to_string()));
			}

			if !message_response.has_more.unwrap_or(false) {
				break;
			}

			cursor = message_response.response_metadata.unwrap().next_cursor;

			if cursor.is_none() {
				break;
			}
		}

		Ok(all_messages.into())
	}

	async fn get_slice_stream(
		&self,
		_path: &Path,
		_range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let session = self.client.open_session(&self.token);
		let channel_id: SlackChannelId = SlackChannelId::new(self.channel.clone());
		let mut cursor: Option<SlackCursorId> =
			Some(SlackCursorId::new(self.cursor.clone().unwrap_or("".to_string())));
		let latest: SlackTs = SlackTs::new(self.latest.clone().unwrap_or("".to_string()));
		let oldest: SlackTs = SlackTs::new(self.oldest.clone().unwrap_or("".to_string()));
		let mut all_messages = String::new();

		loop {
			let req: SlackApiConversationsHistoryRequest = SlackApiConversationsHistoryRequest {
				channel: Some(channel_id.clone()),
				cursor: cursor.clone(),
				latest: Some(latest.clone()),
				limit: Some(self.limit.unwrap_or(100 as u16)),
				oldest: Some(oldest.clone()),
				inclusive: Some(self.inclusive.unwrap_or(true)),
			};
			let message_response = session.conversations_history(&req).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while getting the messages: {:?}", err).into(),
				)
			})?;

			for messages in message_response.messages {
				all_messages.push_str(&messages.content.text.unwrap_or("".to_string()));
			}

			if !message_response.has_more.unwrap_or(false) {
				break;
			}

			cursor = message_response.response_metadata.unwrap().next_cursor;

			if cursor.is_none() {
				break;
			}
		}
		Ok(Box::new(string_to_async_read(all_messages)))
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		let session = self.client.open_session(&self.token);
		let channel_id: SlackChannelId = SlackChannelId::new(self.channel.clone());
		let mut cursor: Option<SlackCursorId> =
			Some(SlackCursorId::new(self.cursor.clone().unwrap_or("".to_string())));
		let latest: SlackTs = SlackTs::new(self.latest.clone().unwrap_or("".to_string()));
		let oldest: SlackTs = SlackTs::new(self.oldest.clone().unwrap_or("".to_string()));
		let mut all_messages = String::new();

		loop {
			let req: SlackApiConversationsHistoryRequest = SlackApiConversationsHistoryRequest {
				channel: Some(channel_id.clone()),
				cursor: cursor.clone(),
				latest: Some(latest.clone()),
				limit: Some(self.limit.unwrap_or(100 as u16)),
				oldest: Some(oldest.clone()),
				inclusive: Some(self.inclusive.unwrap_or(true)),
			};
			let message_response = session.conversations_history(&req).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while getting the messages: {:?}", err).into(),
				)
			})?;

			for messages in message_response.messages {
				all_messages.push_str(&messages.content.text.unwrap_or("".to_string()));
			}

			if !message_response.has_more.unwrap_or(false) {
				break;
			}

			cursor = message_response.response_metadata.unwrap().next_cursor;

			if cursor.is_none() {
				break;
			}
		}
		Ok(all_messages.into())
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		let session = self.client.open_session(&self.token);
		let channel_id: SlackChannelId = SlackChannelId::new(self.channel.clone());
		let mut cursor: Option<SlackCursorId> =
			Some(SlackCursorId::new(self.cursor.clone().unwrap_or("".to_string())));
		let latest: SlackTs = SlackTs::new(self.latest.clone().unwrap_or("".to_string()));
		let oldest: SlackTs = SlackTs::new(self.oldest.clone().unwrap_or("".to_string()));
		let mut all_messages = String::new();

		loop {
			let req: SlackApiConversationsHistoryRequest = SlackApiConversationsHistoryRequest {
				channel: Some(channel_id.clone()),
				cursor: cursor.clone(),
				latest: Some(latest.clone()),
				limit: Some(self.limit.unwrap_or(100 as u16)),
				oldest: Some(oldest.clone()),
				inclusive: Some(self.inclusive.unwrap_or(true)),
			};
			let message_response = session.conversations_history(&req).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while getting the messages: {:?}", err).into(),
				)
			})?;

			for messages in message_response.messages {
				all_messages.push_str(&messages.content.text.unwrap_or("".to_string()));
			}

			if !message_response.has_more.unwrap_or(false) {
				break;
			}

			cursor = message_response.response_metadata.unwrap().next_cursor;

			if cursor.is_none() {
				break;
			}
		}
		Ok(all_messages.len() as u64)
	}

	async fn copy_to(&self, _path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let session = self.client.open_session(&self.token);
		let channel_id: SlackChannelId = SlackChannelId::new(self.channel.clone());
		let mut cursor: Option<SlackCursorId> =
			Some(SlackCursorId::new(self.cursor.clone().unwrap_or("".to_string())));
		let latest: SlackTs = SlackTs::new(self.latest.clone().unwrap_or("".to_string()));
		let oldest: SlackTs = SlackTs::new(self.oldest.clone().unwrap_or("".to_string()));
		let mut all_messages = String::new();

		loop {
			let req: SlackApiConversationsHistoryRequest = SlackApiConversationsHistoryRequest {
				channel: Some(channel_id.clone()),
				cursor: cursor.clone(),
				latest: Some(latest.clone()),
				limit: Some(self.limit.unwrap_or(100 as u16)),
				oldest: Some(oldest.clone()),
				inclusive: Some(self.inclusive.unwrap_or(true)),
			};
			let message_response = session.conversations_history(&req).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while getting the messages: {:?}", err).into(),
				)
			})?;

			for messages in message_response.messages {
				all_messages.push_str(&messages.content.text.unwrap_or("".to_string()));
			}

			if !message_response.has_more.unwrap_or(false) {
				break;
			}

			cursor = message_response.response_metadata.unwrap().next_cursor;

			if cursor.is_none() {
				break;
			}
		}

		let mut body_stream_reader = BufReader::new(all_messages.as_bytes());
		tokio::io::copy_buf(&mut body_stream_reader, output).await?;
		Ok(())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let session = self.client.open_session(&self.token);
		let channel_id: SlackChannelId = SlackChannelId::new(self.channel.clone());
		let mut cursor: Option<SlackCursorId> =
			Some(SlackCursorId::new(self.cursor.clone().unwrap_or("".to_string())));
		let latest: SlackTs = SlackTs::new(self.latest.clone().unwrap_or("".to_string()));
		let oldest: SlackTs = SlackTs::new(self.oldest.clone().unwrap_or("".to_string()));
		let mut all_messages: VecDeque<String> = VecDeque::new();
		let mut all_messages_id: VecDeque<String> = VecDeque::new();

		loop {
			let req: SlackApiConversationsHistoryRequest = SlackApiConversationsHistoryRequest {
				channel: Some(channel_id.clone()),
				cursor: cursor.clone(),
				latest: Some(latest.clone()),
				limit: Some(self.limit.unwrap_or(100 as u16)),
				oldest: Some(oldest.clone()),
				inclusive: Some(self.inclusive.unwrap_or(true)),
			};
			let message_response = session.conversations_history(&req).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while getting the messages: {:?}", err).into(),
				)
			})?;

			for messages in message_response.messages {
				all_messages.extend(messages.content.text);

				let message_id = messages
					.origin
					.client_msg_id
					.unwrap_or(SlackClientMessageId::new("".to_string()))
					.to_string();
				all_messages_id.extend(Some(message_id));
			}

			if !message_response.has_more.unwrap_or(false) {
				break;
			}

			cursor = message_response.response_metadata.unwrap().next_cursor;

			if cursor.is_none() {
				break;
			}
		}

		let source_id = self.source_id.clone();
		let stream = stream! {

			for (index, messages) in all_messages.iter().enumerate() {
				let file_name = format!("{}.slack", all_messages_id[index]);
				let file_name_path = Some(PathBuf::from(file_name));

				let doc_source = Some("slack://".to_string());
				let data_len = Some(messages.len());

				yield Ok(CollectedBytes::new(
					file_name_path,
					Some(Box::pin(string_to_async_read(messages.to_string()))),
					true,
					doc_source,
					data_len,
					source_id.clone(),
					None
				))
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

		rustls::crypto::ring::default_provider()
			.install_default()
			.expect("Failed to install rustls crypto provider");
		let slack_config = SlackCollectorConfig {
			access_token: env::var("SLACK_API_TOKEN")
				.map_err(|e| e.to_string())
				.unwrap_or("".to_string()),
			channel_name: env::var("SLACK_CHANNEL_NAME")
				.map_err(|e| e.to_string())
				.unwrap_or("".to_string()),
			cursor: None,
			oldest: None,
			latest: None,
			inclusive: Some(true),
			limit: Some(10),
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
}
