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

use common::EventType;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::{collections::HashMap, sync::Arc};
use storage::Storage;

/// Insight Information.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct InsightInfo {
	/// ID is  namespaced which is used to identify the insight.
	pub id: String,
	/// Insight name.
	pub name: String,
	/// Insight description.
	pub description: String,
	/// Insight version.
	pub version: String,
	/// Is this insight conversational.
	pub conversational: bool,
	/// Insight author.
	pub author: String,
	/// Insight license.
	pub license: String,
	/// Insight icon. // standard icon size for plugins is 32x32
	/// https://iconify.design/
	pub iconify_icon: String,
	/// Insight options
	pub additional_options: HashMap<String, CustomInsightOption>,
	/// Is a premium insight.
	pub premium: bool,
}

/// Possible custom option values for insights.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub enum InsightCustomOptionValue {
	/// Boolean switch.
	Boolean { value: bool },
	/// Numeric slider.
	Number { min: i32, max: i32, step: i32, value: i32 },
	/// Text input field.
	String { value: String, hidden: Option<bool> },
	/// Dropdown select option.
	Option { values: Vec<String>, value: String },
	/// Callback button.
	Button,
}

impl InsightCustomOptionValue {
	/// Get JSON value of the custom option.
	pub fn json_value(&self) -> Value {
		match self {
			InsightCustomOptionValue::Boolean { value } => Value::Bool(*value),
			InsightCustomOptionValue::Number { value, .. } => Value::Number(Number::from(*value)),
			InsightCustomOptionValue::String { value, .. } => Value::String(value.clone()),
			InsightCustomOptionValue::Option { value, .. } => Value::String(value.clone()),
			InsightCustomOptionValue::Button => Value::Null,
		}
	}
}

/// A custom option for insights.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CustomInsightOption {
	/// Unique identifier for the option.
	pub id: String,
	/// Display label for the option.
	pub label: String,
	/// Tooltip text for the option.
	pub tooltip: Option<String>,
	/// Value of the custom option.
	pub value: InsightCustomOptionValue,
	/// Default value of the custom option.
	#[serde(skip)]
	pub default_value: Option<InsightCustomOptionValue>,
}

impl CustomInsightOption {
	/// Create a new custom option.
	pub fn new(
		id: &str,
		label: &str,
		value: InsightCustomOptionValue,
		default_value: Option<InsightCustomOptionValue>,
	) -> CustomInsightOption {
		CustomInsightOption {
			id: id.to_string(),
			label: label.to_string(),
			tooltip: None,
			value,
			default_value,
		}
	}

	/// Add a tooltip to the custom option.
	pub fn tooltip(mut self, tooltip: &str) -> CustomInsightOption {
		self.tooltip = Some(tooltip.to_string());
		self
	}
}

/// A collection of custom insight options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[repr(C)]
pub struct CustomInsightOptions {
	/// List of custom options.
	pub options: Vec<CustomInsightOption>,
}

impl CustomInsightOptions {
	/// Create a new empty instance.
	pub fn new() -> CustomInsightOptions {
		CustomInsightOptions { options: vec![] }
	}

	/// Add a new custom option.
	pub fn add(
		mut self,
		id: &str,
		label: &str,
		value: InsightCustomOptionValue,
		default_value: Option<InsightCustomOptionValue>,
	) -> CustomInsightOptions {
		self.options.push(CustomInsightOption::new(id, label, value, default_value));
		self
	}

	/// Add a new custom option with a tooltip.
	pub fn add_tooltip(
		mut self,
		id: &str,
		label: &str,
		tooltip: &str,
		value: InsightCustomOptionValue,
		default_value: Option<InsightCustomOptionValue>,
	) -> CustomInsightOptions {
		self.options
			.push(CustomInsightOption::new(id, label, value, default_value).tooltip(tooltip));
		self
	}

	/// Convert all custom options to a JSON object.
	pub fn to_json(&self) -> Value {
		let mut map = serde_json::Map::new();
		for option in &self.options {
			map.insert(option.id.clone(), option.value.json_value());
		}
		Value::Object(map)
	}

	/// Find a custom option by its ID.
	pub fn find_by_id(&self, id: &str) -> Option<&CustomInsightOption> {
		self.options.iter().find(|option| option.id == id)
	}
}

/// Response from Config callback
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
#[repr(C)]
pub enum ConfigCallbackResponse {
	UpdateConfig { config: Value },
	Error { error: String },
	Empty,
}

#[derive(Clone)]
pub struct InsightConfig {
	pub id: String,
	pub discovery_session_id: String,
	pub semantic_pipeline_id: String,
	pub event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub additional_options: HashMap<String, CustomInsightOption>,
}

impl InsightConfig {
	pub fn new(
		id: String,
		discovery_session_id: String,
		semantic_pipeline_id: String,
		event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
		index_storages: Vec<Arc<dyn Storage>>,
		additional_options: HashMap<String, CustomInsightOption>,
	) -> InsightConfig {
		InsightConfig {
			id,
			discovery_session_id,
			semantic_pipeline_id,
			event_storages,
			index_storages,
			additional_options,
		}
	}

	pub fn get_custom_option(&self, id: &str) -> Option<&CustomInsightOption> {
		self.additional_options.get(id)
	}
}
