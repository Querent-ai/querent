use std::{collections::HashMap, hash::Hash};

use candle_core::{Result, Tensor};
use candle_nn::{Activation, Dropout, Init, Module, VarBuilder};

use super::{
	utils::{apply, mean_agg},
	HeteroGnnModule,
};

/// https://arxiv.org/pdf/1703.06103.pdf
/// - Relation-Based Transformation
/// - Root is also transformed
pub struct HeteroGcnConv<NodeType, EdgeType> {
	node_ws: HashMap<NodeType, Tensor>,
	edge_ws: HashMap<(NodeType, EdgeType, NodeType), Tensor>,
}
impl<NodeType, EdgeType> HeteroGcnConv<NodeType, EdgeType>
where
	NodeType: Clone + Eq + Hash + ToString,
	EdgeType: Clone + Eq + Hash + ToString,
{
	pub fn new(
		in_dims: &[(NodeType, usize)],
		out_dims: &[(NodeType, usize)],
		edge_types: &[(NodeType, EdgeType, NodeType)],
		vs: VarBuilder,
	) -> Result<Self> {
		let in_dims: HashMap<NodeType, usize> = in_dims.iter().cloned().collect();
		let out_dims: HashMap<NodeType, usize> = out_dims.iter().cloned().collect();

		let mut node_ws = HashMap::new();
		for (node_type, &out_dim) in &out_dims {
			let in_dim = in_dims[node_type];

			let bound = (6.0 / (in_dim + out_dim) as f64).sqrt();
			let weight = vs.get_with_hints(
				(in_dim, out_dim),
				&format!("weight[{}]", node_type.to_string()),
				Init::Uniform { lo: -bound, up: bound },
			)?;
			node_ws.insert(node_type.clone(), weight);
		}
		let mut edge_ws = HashMap::new();
		for edge_type in edge_types.iter() {
			if !in_dims.contains_key(&edge_type.2) || !out_dims.contains_key(&edge_type.0) {
				continue;
			}
			let in_dim = in_dims[&edge_type.2];
			let out_dim = out_dims[&edge_type.0];

			let bound = (6.0 / (in_dim + out_dim) as f64).sqrt();
			let weight = vs.get_with_hints(
				(in_dim, out_dim),
				&format!(
					"weight[{},{},{}]",
					edge_type.0.to_string(),
					edge_type.1.to_string(),
					edge_type.2.to_string()
				),
				Init::Uniform { lo: -bound, up: bound },
			)?;
			edge_ws.insert(edge_type.clone(), weight);
		}
		Ok(Self { node_ws, edge_ws })
	}

	pub fn forward(
		&self,
		xs: &HashMap<NodeType, Tensor>,
		edge_index: &HashMap<(NodeType, EdgeType, NodeType), Tensor>,
	) -> Result<HashMap<NodeType, Tensor>> {
		let mut output: HashMap<NodeType, Tensor> = self
			.node_ws
			.iter()
			.map(|(node_type, ws)| xs[node_type].matmul(ws).map(|x| (node_type.clone(), x)))
			.collect::<Result<_>>()?;
		for (edge_type, ws) in &self.edge_ws {
			let edge_index = &edge_index[edge_type];
			output.insert(
				edge_type.0.clone(),
				mean_agg(&xs[&edge_type.2].matmul(ws)?, edge_index, &output[&edge_type.0])?,
			);
		}
		Ok(output)
	}
}

pub struct HeteroGcn<NodeType, EdgeType> {
	layers: Vec<HeteroGcnConv<NodeType, EdgeType>>,
	activation_fn: Activation,
	dropout: Dropout,
	first_activation: bool,
}
impl<NodeType, EdgeType> HeteroGnnModule<NodeType, EdgeType> for HeteroGcn<NodeType, EdgeType>
where
	NodeType: Clone + Eq + Hash + ToString,
	EdgeType: Clone + Eq + Hash + ToString,
{
	fn forward_t(
		&self,
		xs: &HashMap<NodeType, Tensor>,
		edge_index: &HashMap<(NodeType, EdgeType, NodeType), Tensor>,
		train: bool,
	) -> Result<HashMap<NodeType, Tensor>> {
		let mut xs = xs.clone();
		for (idx, conv) in self.layers.iter().enumerate() {
			if idx >= 1 || self.first_activation {
				xs = apply(&xs, |x| {
					self.activation_fn.forward(x).and_then(|x| self.dropout.forward(&x, train))
				})?;
			}
			xs = conv.forward(&xs, edge_index)?;
		}
		Ok(xs)
	}
}

pub fn hetero_gcn<NodeType, EdgeType>(
	layer_sizes: &[&[(NodeType, usize)]],
	edge_types: &[(NodeType, EdgeType, NodeType)],
	vs: VarBuilder,
) -> Result<HeteroGcn<NodeType, EdgeType>>
where
	NodeType: Clone + Eq + Hash + ToString,
	EdgeType: Clone + Eq + Hash + ToString,
{
	let num_layers = layer_sizes.len();
	let mut layers = Vec::new();
	for i in 1..num_layers {
		layers.push(HeteroGcnConv::new(
			layer_sizes[i - 1],
			layer_sizes[i],
			edge_types,
			vs.pp(i.to_string()),
		)?);
	}
	Ok(HeteroGcn {
		layers,
		activation_fn: Activation::default(),
		dropout: Dropout::new(0.1),
		first_activation: false,
	})
}

#[cfg(test)]
mod tests {
	use candle_core::{DType, Device};
	use candle_nn::VarMap;

	use super::*;

	#[test]
	fn test_hetero_gcn() -> Result<()> {
		let device = Device::cuda_if_available(0)?;
		let varmap = VarMap::new();
		let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
		let model = hetero_gcn(
			&[&[('a', 1), ('b', 2)], &[('a', 3), ('b', 4)]],
			&[('a', 'x', 'b'), ('b', 'y', 'a')],
			vs.pp("gcn"),
		)?;
		let xs = HashMap::from([
			('a', Tensor::randn(0f32, 1f32, (5, 1), &device)?),
			('b', Tensor::randn(0f32, 1f32, (6, 2), &device)?),
		]);
		let edge_index = HashMap::from([
			(('a', 'x', 'b'), Tensor::from_slice(&[0u32, 1u32], (2, 1), &device)?),
			(('b', 'y', 'a'), Tensor::from_slice(&[2u32, 3u32], (2, 1), &device)?),
		]);
		let output = model.forward(&xs, &edge_index)?;
		assert_eq!(output[&'a'].shape().dims(), &[5, 3]);
		assert_eq!(output[&'b'].shape().dims(), &[6, 4]);
		Ok(())
	}
}
