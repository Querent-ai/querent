pub mod insights;
pub use insights::*;
pub mod types;
pub use types::*;
pub mod x_ai;
pub use x_ai::*;

pub async fn all_insights_info_available() -> Vec<InsightInfo> {
	vec![
		// xplainable insights: GraphRag algorithm to interact with data graph and generate insights
		xplainable::XAI::new().await.info().await,
	]
}
