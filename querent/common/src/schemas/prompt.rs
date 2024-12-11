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

use super::messages::Message;

#[derive(Debug, Clone)]
pub struct PromptValue {
	messages: Vec<Message>,
}
impl PromptValue {
	pub fn from_string(text: &str) -> Self {
		let message = Message::new_human_message(text);
		Self { messages: vec![message] }
	}
	pub fn from_messages(messages: Vec<Message>) -> Self {
		Self { messages }
	}

	pub fn to_string(&self) -> String {
		self.messages
			.iter()
			.map(|m| format!("{}: {}", m.message_type.to_string(), m.content))
			.collect::<Vec<String>>()
			.join("\n")
	}

	pub fn to_chat_messages(&self) -> Vec<Message> {
		self.messages.clone()
	}
}
