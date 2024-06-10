use crate::{Engine, EngineResult};
use async_trait::async_trait;
use crossbeam_channel::Receiver;
use futures::{stream, Stream};
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
		let mut events = Vec::new();

		while let Ok(token) = token_channel.recv() {
			let graph_triple_payload =
				r#"{"subject": "s", "predicate": "p", "object": "o", sentence": "s p o"}"#;

			// Simulate processing each token and generating an event state.
			let event_state = EventState {
				event_type: EventType::Graph,
				doc_source: token.doc_source,
				file: token.file,
				timestamp: 0.0,
				image_id: None,
				payload: graph_triple_payload.to_string(),
			};
			events.push(Ok(event_state));
		}

		// Convert the vector of events to a stream.
		let event_stream = stream::iter(events);
		Ok(Box::pin(event_stream)
			as Pin<Box<dyn Stream<Item = EngineResult<EventState>> + Send + 'static>>)
	}
}

impl MockEngine {
	pub fn new() -> Self {
		MockEngine
	}
}
