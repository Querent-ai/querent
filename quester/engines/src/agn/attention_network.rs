use std::{collections::HashSet, sync::Arc};

use llms::LLM;

#[derive(Clone)]
pub struct AttentionGraphNetwork {
	pub llm: Arc<dyn LLM>,
	pub entities: HashSet<String>,
}

impl AttentionGraphNetwork {
	pub fn new(llm: Arc<dyn LLM>, entities: HashSet<String>) -> Self {
		Self { llm, entities }
	}
}
