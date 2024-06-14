pub mod bert{
	pub mod bert;
pub use bert::*;
pub mod bert_model_functions;
pub use bert_model_functions::*;
// pub mod bert_tokenclassification;
}
pub mod roberta{
	pub mod roberta_model_functions;
	pub mod roberta_utils;
}
pub mod modelling_outputs;
use ordered_float::OrderedFloat;

/// Describes the mean and sigma of distribution of embedding similarity in the embedding space.
///
/// The intended use is to make the similarity score more comparable to the regular ranking score.
/// This allows to correct effects where results are too "packed" around a certain value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DistributionShift {
	/// Value where the results are "packed".
	///
	/// Similarity scores are translated so that they are packed around 0.5 instead
	pub current_mean: OrderedFloat<f32>,

	/// standard deviation of a similarity score.
	///
	/// Set below 0.4 to make the results less packed around the mean, and above 0.4 to make them more packed.
	pub current_sigma: OrderedFloat<f32>,
}

