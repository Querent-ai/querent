// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1). 
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services, 
//    or any service or product offering that provides database, big data, or analytics 
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied, 
// including but not limited to the warranties of merchantability, fitness for a particular purpose, 
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.ai).

pub mod bert {
	pub mod bert;
	pub use bert::*;
	pub mod bert_model_functions;
	pub use bert_model_functions::*;
	// pub mod bert_tokenclassification;
}
pub mod roberta {
	pub mod roberta;
	pub mod roberta_model_functions;
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
