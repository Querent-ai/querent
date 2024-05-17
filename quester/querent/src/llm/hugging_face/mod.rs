use async_openai::{
	types::{
		ChatChoiceStream, ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessageArgs,
		ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
		ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs,
		ChatCompletionToolArgs, ChatCompletionToolType, CreateChatCompletionRequest,
		CreateChatCompletionRequestArgs, FunctionObjectArgs,
	},
	Client,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};

use crate::{
	language_models::{llm::LLM, options::CallOptions, GenerateResult, LLMError, TokenUsage},
	schemas::{
		messages::{Message, MessageType},
		FunctionCallBehavior, StreamData,
	},
};

#[derive(Clone)]
pub enum HuggingFaceModels{
	
}