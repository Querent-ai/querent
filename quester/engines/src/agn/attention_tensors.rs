use async_stream::stream;
use async_trait::async_trait;
use common::{EventState, EventType, SemanticKnowledgePayload};
use futures::{Stream, StreamExt};
use llms::llm::LLM;
use proto::semantics::IngestedTokens;
use std::{pin::Pin, sync::Arc};

use crate::{Engine, EngineResult};

pub struct AttentionTensorsEngine {
	pub llm: Arc<dyn LLM>,
	pub entities: Vec<String>,
}

impl AttentionTensorsEngine {
	pub fn new(llm: Arc<dyn LLM>, entities: Vec<String>) -> Self {
		Self { llm, entities }
	}
}

#[async_trait]
impl Engine for AttentionTensorsEngine {
	async fn process_ingested_tokens(
		&self,
		token_stream: Pin<Box<dyn Stream<Item = IngestedTokens> + Send + 'static>>,
	) -> EngineResult<Pin<Box<dyn Stream<Item = EngineResult<EventState>> + Send + 'static>>> {
		let stream = stream! {
			let mut token_stream = token_stream;
			while let Some(token) = token_stream.next().await {
				// create a payload
				let payload = SemanticKnowledgePayload {
					subject: "mock".to_string(),
					subject_type: "mock".to_string(),
					predicate: "mock".to_string(),
					predicate_type: "mock".to_string(),
					object: "mock".to_string(),
					object_type: "mock".to_string(),
					sentence: "mock".to_string(),
					image_id: None,
				};

				// create an event
				let event = EventState {
					event_type: EventType::Graph,
					file: token.file,
					doc_source: token.doc_source,
					image_id: None,
					timestamp: 0.0,
					payload: serde_json::to_string(&payload).unwrap_or_default(),
				};
				yield Ok(event);
			}
		};
		Ok(Box::pin(stream))
	}
}
