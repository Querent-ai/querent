use async_trait::async_trait;
use proto::semantics::IngestedTokens;
use regex::Regex;

use crate::{AsyncProcessor, IngestorResult};

pub struct TextCleanupProcessor;

impl TextCleanupProcessor {
	pub fn new() -> Self {
		Self {}
	}

	async fn cleanup_text(&self, data: &str) -> String {
		let mut data = data.to_string();

		data = data.replace("\"", "").replace('“', "").replace('”', "");
		data = data.replace("\\n", " ").replace("\\t", " ");

		let re_hex = Regex::new(r"\\x[0-9a-fA-F]{2}").unwrap();
		data = re_hex.replace_all(&data, "").to_string();

		let re_control = Regex::new(r"[\x00-\x1F\x7F]+").unwrap();
		data = re_control.replace_all(&data, "").to_string();
		let re_whitespace = Regex::new(r"\s+").unwrap();
		data = re_whitespace.replace_all(&data, " ").to_string();

		data = data.chars().filter(|&c| c.is_ascii() || !c.is_control()).collect();

		data
	}
}

#[async_trait]
impl AsyncProcessor for TextCleanupProcessor {
	async fn process_text(&self, data: IngestedTokens) -> IngestorResult<IngestedTokens> {
		let tokens = data.clone().data.clone();
		let mut cleaned_tokens = Vec::new();
		for token in tokens.iter() {
			let cleaned_text = self.cleanup_text(&token).await;
			cleaned_tokens.push(cleaned_text.clone());
		}

		let mut data = data.clone();
		data.data = cleaned_tokens;
		Ok(data)
	}
}
