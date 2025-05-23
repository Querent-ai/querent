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

use crate::schemas::{memory::BaseMemory, messages::Message};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SimpleMemory {
	messages: Vec<Message>,
}

impl SimpleMemory {
	pub fn new() -> Self {
		Self { messages: Vec::new() }
	}
}

impl Into<Arc<dyn BaseMemory>> for SimpleMemory {
	fn into(self) -> Arc<dyn BaseMemory> {
		Arc::new(self)
	}
}

impl Into<Arc<Mutex<dyn BaseMemory>>> for SimpleMemory {
	fn into(self) -> Arc<Mutex<dyn BaseMemory>> {
		Arc::new(Mutex::new(self))
	}
}

impl BaseMemory for SimpleMemory {
	fn messages(&self) -> Vec<Message> {
		self.messages.clone()
	}
	fn add_message(&mut self, message: Message) {
		self.messages.push(message);
	}
	fn clear(&mut self) {
		self.messages.clear();
	}
}
