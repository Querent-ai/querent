use std::collections::HashMap;

use querent_synapse::comm::{MessageState, MessageType};

#[derive(Clone, Debug)]
pub struct MessageStateBatches {
	pub pipeline_id: String,
	pub message_state_batches: HashMap<MessageType, Vec<MessageState>>,
}
