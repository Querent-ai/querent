pub mod insights;
use std::collections::HashMap;

pub use insights::*;
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
