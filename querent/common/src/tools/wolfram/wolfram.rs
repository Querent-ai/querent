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

use async_trait::async_trait;
use serde_json::Value;

use crate::tools::Tool;
use std::error::Error;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WolframError {
	code: String,
	msg: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum WolframErrorStatus {
	Error(WolframError),
	NoError(bool),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WolframResponse {
	queryresult: WolframResponseContent,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WolframResponseContent {
	success: bool,
	error: WolframErrorStatus,
	pods: Option<Vec<Pod>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Pod {
	title: String,
	subpods: Vec<Subpod>,
}

impl From<Pod> for String {
	fn from(pod: Pod) -> String {
		let subpods_str: Vec<String> = pod
			.subpods
			.into_iter()
			.map(|subpod| String::from(subpod))
			.filter(|s| !s.is_empty())
			.collect();

		if subpods_str.is_empty() {
			return String::from("");
		}

		format!("{{\"title\": {},\"subpods\": [{}]}}", pod.title, subpods_str.join(","))
	}
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Subpod {
	title: String,
	plaintext: String,
}

impl From<Subpod> for String {
	fn from(subpod: Subpod) -> String {
		if subpod.plaintext.is_empty() {
			return String::from("");
		}

		format!(
			"{{\"title\": \"{}\",\"plaintext\": \"{}\"}}",
			subpod.title,
			subpod.plaintext.replace("\n", " // ")
		)
	}
}

/// When being used within agents GPT4 is recommended
pub struct Wolfram {
	app_id: String,
	exclude_pods: Vec<String>,
	client: reqwest::Client,
}

impl Wolfram {
	pub fn new(app_id: String) -> Self {
		Self { app_id, exclude_pods: Vec::new(), client: reqwest::Client::new() }
	}

	pub fn with_excludes<S: AsRef<str>>(mut self, exclude_pods: &[S]) -> Self {
		self.exclude_pods = exclude_pods.iter().map(|s| s.as_ref().to_owned()).collect();
		self
	}

	pub fn with_app_id<S: AsRef<str>>(mut self, app_id: S) -> Self {
		self.app_id = app_id.as_ref().to_owned();
		self
	}
}

impl Default for Wolfram {
	fn default() -> Wolfram {
		Wolfram {
			app_id: std::env::var("WOLFRAM_APP_ID").unwrap_or_default(),
			exclude_pods: Vec::new(),
			client: reqwest::Client::new(),
		}
	}
}

#[async_trait]
impl Tool for Wolfram {
	fn name(&self) -> String {
		String::from("Wolfram")
	}

	fn description(&self) -> String {
		String::from(
			"Wolfram Solver leverages the Wolfram Alpha computational engine
            to solve complex queries. Input should be a valid mathematical 
            expression or query formulated in a way that Wolfram Alpha can 
            interpret.",
		)
	}
	async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
		let input = input.as_str().ok_or("Invalid input")?;
		let mut url = format!(
            "https://api.wolframalpha.com/v2/query?appid={}&input={}&output=JSON&format=plaintext&podstate=Result__Step-by-step+solution",
            &self.app_id,
            urlencoding::encode(input)
        );

		if !self.exclude_pods.is_empty() {
			url += &format!("&excludepodid={}", self.exclude_pods.join(","));
		}

		let response: WolframResponse = self.client.get(&url).send().await?.json().await?;

		if let WolframErrorStatus::Error(error) = response.queryresult.error {
			return Err(Box::new(std::io::Error::new(
				std::io::ErrorKind::Other,
				format!("Wolfram Error {}: {}", error.code, error.msg),
			)));
		} else if !response.queryresult.success {
			return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Wolfram Error invalid query input: The query requested can not be processed by Wolfram"),
            )));
		}

		let pods_str: Vec<String> = response
			.queryresult
			.pods
			.unwrap_or_default()
			.into_iter()
			.map(|s| String::from(s))
			.filter(|s| !s.is_empty())
			.collect();

		Ok(format!("{{\"pods\": [{}]}}", pods_str.join(",")))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	#[ignore]
	async fn test_wolfram() {
		let wolfram = Wolfram::default().with_excludes(&vec!["Plot"]);
		let input = "Solve x^2 - 2x + 1 = 0";
		let result = wolfram.call(input).await;

		assert!(result.is_ok());
		println!("{}", result.unwrap());
	}
}
