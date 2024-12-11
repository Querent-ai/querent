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

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub enum ToolInput {
	//Will implement this in the future
	StrInput(String),
	DictInput(HashMap<String, String>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentAction {
	pub tool: String,
	pub tool_input: String, //this should be ToolInput in the future
	pub log: String,
}

///Log tools is a struct used by the openai-like agents
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogTools {
	pub tool_id: String,
	pub tools: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentFinish {
	pub output: String,
}

pub enum AgentEvent {
	Action(Vec<AgentAction>),
	Finish(AgentFinish),
}

pub enum AgentPlan {
	Text(AgentEvent),
	Stream(mpsc::Receiver<Result<String, reqwest_eventsource::Error>>),
}
