use async_trait::async_trait;
use querent_synapse::comm::IngestedTokens;
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

		data = data.replace("\n", " ").replace("\r", " ").replace("\t", " ");

		let re_control = Regex::new(r"[\x00-\x08\x0b\x0c\x0e-\x1f]+").unwrap();
		data = re_control.replace_all(&data, "").to_string();

		data
	}
}

#[async_trait]
impl AsyncProcessor for TextCleanupProcessor {
	async fn process_text(&self, data: IngestedTokens) -> IngestorResult<IngestedTokens> {
		let tokens = data.clone().data.clone().unwrap_or_default();
		let mut cleaned_tokens = Vec::new();
		for token in tokens.iter() {
			let cleaned_text = self.cleanup_text(&token).await;
			cleaned_tokens.push(cleaned_text.clone());
		}

		let mut data = data.clone();
		data.data = Some(cleaned_tokens);
		Ok(data)
	}
}
