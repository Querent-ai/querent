use crate::layers::{LayersResult, NNLayer};
use petgraph::{
	algo::{all_simple_paths, dijkstra},
	graph::{Graph, NodeIndex},
};
use std::ops::Mul;
use tch::{nn, nn::OptimizerConfig, Device, Kind, Tensor};

/// A simple neural network model for link prediction.
pub struct LinkPredictionModel {
	pub device: Device,
	pub vs: nn::VarStore,
	pub optimizer: nn::Optimizer,
}

impl LinkPredictionModel {
	pub fn new(device: Device) -> Self {
		let vs = nn::VarStore::new(device);
		let optimizer = nn::Adam::default().build(&vs, 1e-3).unwrap();
		Self { device, vs, optimizer }
	}
}

impl NNLayer for LinkPredictionModel {
	/// Encodes node embeddings (basic identity function for now).
	fn encode(
		&self,
		_input: &Tensor,
		_edge_index: &Tensor,
		_edge_attr: Option<&Tensor>,
	) -> LayersResult<Option<&Tensor>> {
		Ok(None)
	}

	/// Decodes edge probabilities using a dot product.
	fn decode(&self, z: &Tensor, edge_index: &Tensor) -> LayersResult<Tensor> {
		let indices = edge_index.to_kind(Kind::Int64);
		let src = z.index_select(0, &indices.get(0));
		let dst = z.index_select(0, &indices.get(1));
		let scores = src.mul(&dst).sum_dim_intlist(&[1i64][..], false, Kind::Float);
		Ok(scores)
	}

	/// Performs a single training step with Adam optimizer.
	fn train(&mut self, input: &Tensor, edge_index: &Tensor, target: &Tensor) -> LayersResult<f64> {
		self.optimizer.zero_grad();

		let embeddings = self.encode(input, edge_index, None)?;
		let embeddings = embeddings.unwrap_or(input);
		let predictions = self.decode(embeddings, edge_index)?;

		let loss = (predictions - target).pow_tensor_scalar(2).mean(Kind::Float);
		loss.backward();
		self.optimizer.step();

		Ok(loss.double_value(&[]))
	}

	/// Computes the reconstruction loss.
	fn compute_loss(
		&self,
		z: &Tensor,
		pos_edges: &Tensor,
		neg_edges: &Tensor,
	) -> LayersResult<f64> {
		let pos_scores = self.decode(z, pos_edges)?;
		let neg_scores = self.decode(z, neg_edges)?;
		let loss = pos_scores.mean(Kind::Float) - neg_scores.mean(Kind::Float);
		Ok(loss.double_value(&[]))
	}

	/// Predicts missing links using negative sampling.
	fn predict(
		&self,
		z: &Tensor,
		_edge_index: &Tensor,
		num_samples: usize,
	) -> LayersResult<Vec<(usize, usize, f64)>> {
		let num_nodes = z.size()[0] as usize;
		let mut predictions = Vec::new();

		for _ in 0..num_samples {
			let i = rand::random::<usize>() % num_nodes;
			let j = rand::random::<usize>() % num_nodes;
			let score = self.decode(
				z,
				&Tensor::f_from_slice(&[i as i64, j as i64]).unwrap().reshape(&[2, 1]),
			)?;
			predictions.push((i, j, score.double_value(&[])));
		}
		Ok(predictions)
	}

	/// Evaluates link prediction using AUC and Precision-Recall AUC.
	fn evaluate(&self, pos_probs: &Tensor, neg_probs: &Tensor) -> LayersResult<(f64, f64)> {
		let pos_scores = pos_probs.to_device(Device::Cpu).contiguous();
		let neg_scores = neg_probs.to_device(Device::Cpu).contiguous();

		// Convert tensors to Vec<f64> for ranking
		let mut pos_values: Vec<f64> = Vec::new();
		for i in 0..pos_scores.size()[0] {
			pos_values.push(pos_scores.double_value(&[i as i64]));
		}
		let mut neg_values: Vec<f64> = Vec::new();
		for i in 0..neg_scores.size()[0] {
			neg_values.push(neg_scores.double_value(&[i as i64]));
		}

		// Compute AUC
		let auc = compute_auc(&pos_values, &neg_values);

		// Compute PR-AUC
		let pr_auc = compute_pr_auc(&pos_values, &neg_values);

		Ok((auc, pr_auc))
	}

	/// Multi-hop reasoning using Dijkstra's shortest path.
	fn multi_hop_reasoning(
		&self,
		graph: &Graph<usize, ()>,
		src: NodeIndex,
		tgt: NodeIndex,
	) -> LayersResult<Option<Vec<NodeIndex>>> {
		let costs = dijkstra(graph, src, None, |_| 1);
		let path = costs.get(&tgt).map(|_| {
			let paths = all_simple_paths::<Vec<NodeIndex>, _>(graph, src, tgt, 1, None)
				.next()
				.unwrap_or_default();
			paths
		});
		Ok(path)
	}

	/// Generates insights from predicted links.
	fn generate_insights(&self, subject: &str, object: &str, contexts: &[String]) -> String {
		format!("Possible relation between '{}' and '{}': {:?}", subject, object, contexts)
	}

	/// Sets the computation device (CPU/GPU).
	fn set_device(&mut self, device: Device) -> LayersResult<()> {
		self.device = device;
		Ok(())
	}
}

/// Computes the AUC (Area Under the Curve).
fn compute_auc(pos_scores: &[f64], neg_scores: &[f64]) -> f64 {
	let mut correct = 0.0;
	let mut total = 0;

	for &p in pos_scores {
		for &n in neg_scores {
			if p > n {
				correct += 1.0;
			} else if p == n {
				correct += 0.5;
			}
			total += 1;
		}
	}

	if total == 0 {
		return 0.5; // Random guessing baseline
	}
	correct as f64 / total as f64
}

/// Computes the Precision-Recall AUC (PR-AUC).
fn compute_pr_auc(pos_scores: &[f64], neg_scores: &[f64]) -> f64 {
	let mut scores: Vec<(f64, bool)> = pos_scores.iter().map(|&s| (s, true)).collect();
	scores.extend(neg_scores.iter().map(|&s| (s, false)));

	// Sort by score in descending order
	scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

	let mut tp = 0.0;
	let mut fp = 0.0;
	let mut prev_precision = 1.0;
	let mut prev_recall = 0.0;
	let mut auc = 0.0;

	let total_positives = pos_scores.len() as f64;

	for &(_, is_positive) in &scores {
		if is_positive {
			tp += 1.0;
		} else {
			fp += 1.0;
		}

		let precision = tp / (tp + fp);
		let recall = tp / total_positives;

		// Trapezoidal integration
		auc += (recall - prev_recall) * (precision + prev_precision) / 2.0;

		prev_precision = precision;
		prev_recall = recall;
	}

	auc
}
