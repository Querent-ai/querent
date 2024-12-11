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

use std::ops::Deref;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tools::Tool;

#[derive(Clone, Copy, Debug)]
pub enum FunctionCallBehavior {
	None,
	Auto,
}

#[derive(Clone, Debug)]
pub struct FunctionDefinition {
	pub name: String,
	pub description: String,
	pub parameters: Value,
}

impl FunctionDefinition {
	pub fn new(name: &str, description: &str, parameters: Value) -> Self {
		FunctionDefinition {
			name: name.trim().replace(" ", "_"),
			description: description.to_string(),
			parameters,
		}
	}

	/// Generic function that can be used with both Arc<Tool>, Box<Tool>, and direct references
	pub fn from_langchain_tool<T>(tool: &T) -> FunctionDefinition
	where
		T: Deref<Target = dyn Tool> + ?Sized,
	{
		FunctionDefinition {
			name: tool.name().trim().replace(" ", "_"),
			description: tool.description(),
			parameters: tool.parameters(),
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionCallResponse {
	pub id: String,
	#[serde(rename = "type")]
	pub type_field: String,
	pub function: FunctionDetail,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionDetail {
	pub name: String,
	///this should be an string, and this should be passed to the tool, to
	///then be deserilised inside the tool, becuase just the tools knows the names of the arguments.
	pub arguments: String,
}

impl FunctionCallResponse {
	pub fn from_str(s: &str) -> Result<Self, serde_json::Error> {
		serde_json::from_str(s)
	}
}
