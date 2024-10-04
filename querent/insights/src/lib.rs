pub mod insights;
pub use insights::*;
use std::{collections::HashMap, sync::Arc};
pub mod types;
pub use types::*;
pub mod x_ai;
pub use x_ai::*;
pub mod transfer_learning;
pub use transfer_learning::*;
pub mod insight_utils;
pub use insight_utils::*;
pub mod anomaly_detection;
pub use anomaly_detection::*;
pub mod cross_document_summarization;
pub use cross_document_summarization::*;
pub mod report_generation;
pub use report_generation::*;
pub mod graph_builder;
pub use graph_builder::*;

pub async fn all_insights_info_available() -> Vec<InsightInfo> {
	vec![
		// xplainable insights: GraphRag algorithm to interact with data graph and generate insights
		xplainable_openai::XAI::new().info().await,
		// xplainable insights: GraphRag algorithm to interact with data graph and generate insights
		xplainable_claude::XAIClaude::new().info().await,
		// xplainable insights: GraphRag algorithm to interact with data graph and generate insights
		xplainable_ollama::XAIOllama::new().info().await,
		// transfer learning insights: Transfer learning insights
		tl_insight::TLV1::new().info().await,
		// Anomaly detection insights: Anomaly detection insights
		anomaly_insights::ADV1::new().info().await,
		// Cross Document Summarization: Cross Document Summarization
		cds_insight::CDSV1::new().info().await,
		// Report Generation: Report Generation
		report_generation_insight::RGV1::new().info().await,
		// Graph Builder: Graph Builder
		gb_insight::GBV1::new().info().await,
	]
}

pub async fn map_insights_id_vs_info() -> HashMap<String, InsightInfo> {
	let insights = all_insights_info_available().await;
	let mut map = HashMap::new();
	for insight in insights {
		map.insert(insight.id.clone(), insight);
	}
	map
}

pub async fn get_insight_info_by_id(insight_id: &str) -> Option<InsightInfo> {
	let insights = all_insights_info_available().await;
	for insight in insights {
		if insight.id == insight_id {
			return Some(insight);
		}
	}
	None
}

pub async fn get_insight_runner_by_id(insight_id: &str) -> Option<Arc<dyn Insight>> {
	match insight_id {
		"querent.insights.x_ai.openai" => {
			let insight = xplainable_openai::XAI::new();
			Some(Arc::new(insight))
		},
		"querent.insights.x_ai.claude" => {
			let insight = xplainable_claude::XAIClaude::new();
			Some(Arc::new(insight))
		},
		"querent.insights.x_ai.ollama" => {
			let insight = xplainable_ollama::XAIOllama::new();
			Some(Arc::new(insight))
		},
		"querent.insights.graph_builder.gbv1" => {
			let insight = gb_insight::GBV1::new();
			Some(Arc::new(insight))
		},
		_ => None,
	}
}
