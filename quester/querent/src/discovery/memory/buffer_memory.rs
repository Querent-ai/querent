use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{BaseMemory, Message};

pub struct BufferMemory {
	window_size: usize,
	messages: Vec<Message>,
}

impl Default for BufferMemory {
	fn default() -> Self {
		Self::new(10)
	}
}

impl BufferMemory {
	pub fn new(window_size: usize) -> Self {
		Self { messages: Vec::new(), window_size }
	}
}

impl Into<Arc<dyn BaseMemory>> for BufferMemory {
	fn into(self) -> Arc<dyn BaseMemory> {
		Arc::new(self)
	}
}

impl Into<Arc<Mutex<dyn BaseMemory>>> for BufferMemory {
	fn into(self) -> Arc<Mutex<dyn BaseMemory>> {
		Arc::new(Mutex::new(self))
	}
}

impl BaseMemory for BufferMemory {
	fn messages(&self) -> Vec<Message> {
		self.messages.clone()
	}
	fn add_message(&mut self, message: Message) {
		if self.messages.len() >= self.window_size {
			self.messages.remove(0);
		}
		self.messages.push(message);
	}
	fn clear(&mut self) {
		self.messages.clear();
	}
}
