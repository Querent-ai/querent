use async_trait::async_trait;
use common::SemanticKnowledgePayload;
use petgraph::{adj::NodeIndex, graph::Graph, Directed};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

/// Error types for neural network operations.
#[derive(Debug, Clone, Error)]
#[error("{kind:?}: {source}")]
pub struct LayersError {
	pub kind: LayersErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LayersErrorKind {
	Io,
	NotFound,
	ModelError,
	InvalidInput,
}

/// Result type alias.
pub type LayersResult<T> = Result<T, LayersError>;

/// Graph data structure.
#[derive(Clone)]
pub struct GraphData {
	pub graph: Graph<String, (), Directed>,
	pub node_map: HashMap<String, NodeIndex>,
	pub embeddings: HashMap<NodeIndex, Vec<f32>>,
	pub is_directed: bool,
}

/// Training result.
pub struct TrainResult {
	pub final_loss: f32,
	pub epoch_losses: Vec<f32>,
}

/// Predicted relationship with confidence score.
pub struct PredictionResult {
	pub source: String,
	pub target: String,
	pub probability: f32,
}

/// Negative sample structure.
pub struct NegativeSample {
	pub source: usize,
	pub target: usize,
	pub weight: f32,
}

/// Multi-hop reasoning result.
pub struct MultiHopPath {
	pub path: Vec<String>,
	pub confidence: Option<f32>,
}

/// Model format
pub enum ModelFormat {
	Json,
	Binary,
}

/// Async Trait for neural network models supporting link prediction.
#[async_trait]
pub trait GraphExperiment {
	/// Loads the graph data and node mapping.
	async fn load_data(&mut self, data: Vec<SemanticKnowledgePayload>) -> LayersResult<GraphData>;
	/// Computes the embeddings for the graph.
	async fn compute_embeddings(&mut self) -> LayersResult<()>;
	/// Trains the GCN model.
	async fn train(&mut self, epochs: usize, learning_rate: f32) -> LayersResult<TrainResult>;
	/// Predicts missing relationships between nodes.
	async fn predict_links(&self, source: &str, target: &str) -> LayersResult<PredictionResult>;
	/// Generates negative samples for training.
	async fn generate_negative_samples(
		&self,
		num_samples: usize,
	) -> LayersResult<Vec<NegativeSample>>;
	/// Performs multi-hop reasoning.
	async fn multi_hop_reasoning(
		&self,
		source: &str,
		target: &str,
		max_hops: usize,
	) -> LayersResult<MultiHopPath>;
	/// Saves the model to disk.
	async fn save_model(&self, path: &str, format: ModelFormat) -> LayersResult<()>;
	/// Loads the model from disk.
	async fn load_model(&self, path: &str) -> LayersResult<()>;
	/// Generate natural language explanations for the model.
	async fn explain(&self, source: &str, target: &str) -> LayersResult<String>;
}
