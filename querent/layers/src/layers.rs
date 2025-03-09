use petgraph::graph::{Graph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tch::{Device, Tensor};
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
}

/// Result type alias.
pub type LayersResult<T> = Result<T, LayersError>;

/// Trait for neural network models supporting link prediction.
pub trait NNLayer: Sized {
	/// Encodes node embeddings.
	fn encode(
		&self,
		input: &Tensor,
		edge_index: &Tensor,
		edge_attr: Option<&Tensor>,
	) -> LayersResult<Tensor>;

	/// Decodes edge probabilities.
	fn decode(&self, z: &Tensor, edge_index: &Tensor) -> LayersResult<Tensor>;

	/// Performs a single training step.
	fn train(&mut self, input: &Tensor, edge_index: &Tensor, target: &Tensor) -> LayersResult<f64>;

	/// Computes the reconstruction loss.
	fn compute_loss(&self, z: &Tensor, pos_edges: &Tensor, neg_edges: &Tensor)
		-> LayersResult<f64>;

	/// Predicts missing links using negative sampling.
	fn predict(
		&self,
		z: &Tensor,
		edge_index: &Tensor,
		num_samples: usize,
	) -> LayersResult<Vec<(usize, usize, f64)>>;

	/// Evaluates link prediction using AUC and Precision-Recall AUC.
	fn evaluate(&self, pos_probs: &Tensor, neg_probs: &Tensor) -> LayersResult<(f64, f64)>;

	/// Generates insights by analyzing multi-hop reasoning using Petgraph.
	fn multi_hop_reasoning(
		&self,
		graph: &Graph<usize, ()>,
		src: NodeIndex,
		tgt: NodeIndex,
	) -> LayersResult<Option<Vec<NodeIndex>>>;

	/// Generates natural language insights from the predicted links.
	fn generate_insights(&self, subject: &str, object: &str, contexts: &[String]) -> String;

	/// Sets the computation device (CPU/GPU).
	fn set_device(&mut self, device: Device) -> LayersResult<()>;
}
