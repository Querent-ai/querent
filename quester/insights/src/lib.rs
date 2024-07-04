pub mod insights;
pub use insights::*;
use std::{collections::HashMap, sync::Arc};
pub mod types;
pub use types::*;
pub mod x_ai;
pub use x_ai::*;

pub async fn all_insights_info_available() -> Vec<InsightInfo> {
	vec![
		// xplainable insights: GraphRag algorithm to interact with data graph and generate insights
		xplainable_openai::XAI::new().await.info().await,
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
			let insight = xplainable_openai::XAI::new().await;
			Some(insight)
		},
		_ => None,
	}
}
