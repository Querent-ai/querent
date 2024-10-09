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
	SlackApiTokenValue, SlackChannelId, SlackClient, SlackCursorId, SlackTs,
};
use tokio::io::AsyncRead;

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

#[derive(Clone, Debug)]
pub struct SlackApiClient {
	client: SlackClient<SlackClientHyperConnector<HttpsConnector<HttpConnector>>>,
	source_id: String,
	token: SlackApiToken,
	pub channel: String,
	cursor: String,
	latest: String,
	inclusive: bool,
	limit: u16,
	oldest: String,
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
			limit: config.limit as u16,
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
		Ok(2 as u64)
	}

	async fn copy_to(&self, _path: &Path, _output: &mut dyn SendableAsync) -> SourceResult<()> {
		Ok(())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let session = self.client.open_session(&self.token);
		let channel_id: SlackChannelId = SlackChannelId::new(self.channel.clone());
		let mut cursor: Option<SlackCursorId> = Some(SlackCursorId::new(self.cursor.clone()));
		let latest: SlackTs = SlackTs::new(self.latest.clone());
		let oldest: SlackTs = SlackTs::new(self.oldest.clone());
		let mut all_messages: VecDeque<String> = VecDeque::new();

		loop {
			let req: SlackApiConversationsHistoryRequest = SlackApiConversationsHistoryRequest {
				channel: Some(channel_id.clone()),
				cursor: cursor.clone(),
				latest: Some(latest.clone()),
				limit: Some(self.limit),
				oldest: Some(oldest.clone()),
				inclusive: Some(self.inclusive),
			};
			let message_response = session.conversations_history(&req).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while getting the messages: {:?}", err).into(),
				)
			})?;

			for messages in message_response.messages {
				all_messages.extend(messages.content.text);
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


			yield Ok(CollectedBytes::new(
				Some(PathBuf::from("".to_string())),
				Some(Box::pin(string_to_async_read("".to_string()))),
							true,
							Some("".to_string()),
							Some(2),
							source_id,
							None
			))
		};
		Ok(Box::pin(stream))
	}
}
