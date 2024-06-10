use crate::{Engine, EngineResult};
use async_stream::stream;
use async_trait::async_trait;
use crossbeam_channel::Receiver;
use futures::Stream;
use querent_synapse::{
	callbacks::{EventState, EventType},
	comm::IngestedTokens,
};
use std::pin::Pin;

pub struct MockEngine;

#[async_trait]
impl Engine for MockEngine {
	async fn process_ingested_tokens(
		&self,
		token_channel: Receiver<IngestedTokens>,
	) -> EngineResult<Pin<Box<dyn Stream<Item = EngineResult<EventState>> + Send + 'static>>> {
		let stream = stream! {
			for token in token_channel {
				let event = EventState {
					event_type: EventType::Graph,
					file: token.file,
					doc_source: token.doc_source,
					image_id: None,
					timestamp: 0.0,
					payload: r#"{"subject": "mock", "predicate": "mock", "object": "mock", "sentence": "mock"}"#.to_string(),
				};
				yield Ok(event);
			}
		};
		Ok(Box::pin(stream))
	}
}

impl MockEngine {
	pub fn new() -> Self {
		MockEngine
	}
}
