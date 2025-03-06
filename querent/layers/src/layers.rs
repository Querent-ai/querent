use std::collections::HashMap;
use tch::Tensor;

pub trait NNModel {
	// Forward pass of the model
	fn forward(&self, input: &Tensor) -> Tensor;

	// Training step, computes the loss and updates model parameters
	fn train_step(&mut self, input: &Tensor, target: &Tensor) -> f64;

	// Evaluate model on a dataset
	fn evaluate(&self, input: &Tensor, target: &Tensor) -> f64;

	// Perform link prediction (or any task-specific prediction)
	fn predict(&self, input: &Tensor) -> Tensor;

	// Compute metrics for performance evaluation, e.g., AUC, accuracy, etc.
	fn evaluate_metrics(&self, pos_probs: &Tensor, neg_probs: &Tensor);

	// Generate insights or interpretations based on model predictions
	fn generate_insights(&self, data: &HashMap<String, String>, predictions: &Tensor);
}
