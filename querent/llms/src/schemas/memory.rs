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

pub trait BaseMemory: Send + Sync {
	fn messages(&self) -> Vec<Message>;

	// Use a trait object for Display instead of a generic type
	fn add_user_message(&mut self, message: &dyn std::fmt::Display) {
		// Convert the Display trait object to a String and pass it to the constructor
		self.add_message(Message::new_human_message(message.to_string()));
	}

	// Use a trait object for Display instead of a generic type
	fn add_ai_message(&mut self, message: &dyn std::fmt::Display) {
		// Convert the Display trait object to a String and pass it to the constructor
		self.add_message(Message::new_ai_message(message.to_string()));
	}

	fn add_message(&mut self, message: Message);

	fn clear(&mut self);

	fn to_string(&self) -> String {
		self.messages()
			.iter()
			.map(|msg| format!("{}: {}", msg.message_type.to_string(), msg.content))
			.collect::<Vec<String>>()
			.join("\n")
	}
}

impl<M> From<M> for Box<dyn BaseMemory>
where
	M: BaseMemory + 'static,
{
	fn from(memory: M) -> Self {
		Box::new(memory)
	}
}
