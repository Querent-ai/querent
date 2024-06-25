use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use storage::Storage;

/// Insight Information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct InsightInfo {
	/// Insight name.
	pub name: String,
	/// Insight description.
	pub description: String,
	/// Insight version.
	pub version: String,
	/// Insight author.
	pub author: String,
	/// Insight license.
	pub license: String,
	/// Image bytes, use 1:1 aspect ratio, PNG for transparency recommended.
	#[serde(skip)]
	pub icon: &'static [u8],
	/// Insight options
	pub additional_options: HashMap<String, CustomInsightOption>,
}

/// Possible custom option values for insights.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone)]
pub struct InsightConfig {
	pub id: String,
	pub index_storage: Arc<dyn Storage>,
	pub embedded_knowledge_store: Arc<dyn Storage>,
	pub discovered_knowledge_store: Arc<dyn Storage>,
	pub graph_storage: Option<Arc<dyn Storage>>,
	pub additional_options: HashMap<String, CustomInsightOption>,
}
