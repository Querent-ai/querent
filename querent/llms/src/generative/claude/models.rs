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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use serde::{Deserialize, Serialize};

use crate::schemas::{Message, MessageType};

#[derive(Serialize, Deserialize)]
pub(crate) struct ClaudeMessage {
	pub role: String,
	pub content: String,
}
impl ClaudeMessage {
	pub fn new<S: Into<String>>(role: S, content: S) -> Self {
		Self { role: role.into(), content: content.into() }
	}

	pub fn from_message(message: &Message) -> Self {
		match message.message_type {
			MessageType::SystemMessage => Self::new("system", &message.content),
			MessageType::AIMessage => Self::new("assistant", &message.content),
			MessageType::HumanMessage => Self::new("user", &message.content),
			MessageType::ToolMessage => Self::new("tool", &message.content),
		}
	}
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Payload {
	pub model: String,
	pub messages: Vec<ClaudeMessage>,
	pub max_tokens: u32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stream: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stop_sequences: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub temperature: Option<f32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub top_p: Option<f32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub top_k: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ApiResponse {
	pub content: Vec<Content>,
	pub id: String,
	pub model: String,
	pub role: String,
	pub stop_reason: Option<String>,
	pub stop_sequence: Option<serde_json::Value>, // Adjust based on actual stop_sequence type
	#[serde(rename = "type")]
	pub message_type: String,
	pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Content {
	pub text: String,
	#[serde(rename = "type")]
	pub content_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Usage {
	pub input_tokens: u32,
	pub output_tokens: u32,
}
