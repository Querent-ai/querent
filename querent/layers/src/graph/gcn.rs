use candle_core::{DType, Device, IndexOp, Result, Tensor};
use candle_nn::{Activation, Dropout, Init, Module, VarBuilder, VarMap};

use super::{
	utils::{in_degree, out_degree, weighted_sum_agg},
	GnnModule,
};

#[derive(Clone)]
pub struct GcnParams {
	pub dropout_rate: f32,
	pub activation_fn: Activation,
}

impl Default for GcnParams {
	fn default() -> Self {
		Self { dropout_rate: 0.5, activation_fn: Activation::Relu }
	}
}

#[derive(Clone)]
pub struct GcnConv {
	weight: Tensor,
	bias: Tensor,
}

impl GcnConv {
	pub fn new(in_dim: usize, out_dim: usize, vs: VarBuilder) -> Result<Self> {
		let bound = (6.0 / (in_dim + out_dim) as f64).sqrt();
		let weight = vs.get_with_hints(
			(in_dim, out_dim),
			"weight",
			Init::Uniform { lo: -bound, up: bound },
		)?;
		let bias = vs.get_with_hints(out_dim, "bias", Init::Const(0.0))?;
		Ok(Self { weight, bias })
	}
}

impl GnnModule for GcnConv {
	fn forward_t(&self, xs: &Tensor, edge_index: &Tensor, _train: bool) -> Result<Tensor> {
		let out_degree = out_degree(edge_index)?.maximum(1u32)?;
		let in_degree = in_degree(edge_index)?.maximum(1u32)?;
		let edge_weight = out_degree
			.i(&edge_index.i((0, ..))?)?
			.mul(&in_degree.i(&edge_index.i((1, ..))?)?)?
			.to_dtype(xs.dtype())?
			.powf(-0.5)?;
		let xs = xs.matmul(&self.weight)?;
		weighted_sum_agg(&xs, edge_index, &edge_weight, &xs)?.broadcast_add(&self.bias)
	}
}

#[derive(Clone)]
pub struct GCNModel {
	layers: Vec<GcnConv>,
	pub device: Device,
	pub varmap: VarMap,
	pub dropout: Dropout,
	pub act: Activation,
}

impl GCNModel {
	pub fn with_params(device: Device, layer_sizes: &[usize], params: GcnParams) -> Result<Self> {
		let varmap = VarMap::new();
		let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
		let dropout = Dropout::new(params.dropout_rate);
		let act = params.activation_fn;

		let mut layers = Vec::new();
		for i in 0..layer_sizes.len() - 1 {
			layers.push(GcnConv::new(layer_sizes[i], layer_sizes[i + 1], vs.pp(i.to_string()))?);
		}
		Ok(Self { device, varmap, layers, dropout, act })
	}

	pub fn new(layer_sizes: &[usize], device: &Device) -> Result<Self> {
		Self::with_params(device.clone(), layer_sizes, GcnParams::default())
	}
	pub fn parameters(&self) -> Vec<candle_core::Var> {
		self.varmap.all_vars()
	}
}

impl GnnModule for GCNModel {
	fn forward_t(&self, x: &Tensor, edge_index: &Tensor, train: bool) -> Result<Tensor> {
		let mut h = self.layers[0].forward(x, edge_index)?;
		for layer in &self.layers[1..] {
			h = self.dropout.forward(&h, train)?;
			h = self.act.forward(&h)?;
			h = layer.forward(&h, edge_index)?;
		}
		Ok(h)
	}
}
