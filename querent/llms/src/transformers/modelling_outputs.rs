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

use candle_core::Tensor;

/// Output structure for sequence classification tasks.
///
/// This structure holds the results of sequence classification models,
/// including the loss, logits, hidden states, and attention weights.
#[derive(Debug)]
#[records::record]
pub struct SequenceClassifierOutput {
	/// The loss value (optional).
	loss: Option<Tensor>,
	/// The logits (raw predictions) produced by the model.
	logits: Tensor,
	/// The hidden states from the model (optional).
	hidden_states: Option<Tensor>,
	/// The attention weights from the model (optional).
	attentions: Option<Tensor>,
}

/// Output structure for token classification tasks.
///
/// This structure holds the results of token classification models,
/// including the loss, logits, hidden states, and attention weights.
#[derive(Debug)]
#[records::record]
pub struct TokenClassifierOutput {
	/// The loss value (optional).
	loss: Option<Tensor>,
	/// The logits (raw predictions) produced by the model.
	logits: Tensor,
	/// The hidden states from the model (optional).
	hidden_states: Option<Tensor>,
	/// The attention weights from the model (optional).
	attentions: Option<Tensor>,
}

/// Output structure for question answering tasks.
///
/// This structure holds the results of question answering models,
/// including the loss, start logits, end logits, hidden states, and attention weights.
#[derive(Debug)]
#[records::record]
pub struct QuestionAnsweringModelOutput {
	/// The loss value (optional).
	loss: Option<Tensor>,
	/// The logits for the start positions of the answer span.
	start_logits: Tensor,
	/// The logits for the end positions of the answer span.
	end_logits: Tensor,
	/// The hidden states from the model (optional).
	hidden_states: Option<Tensor>,
	/// The attention weights from the model (optional).
	attentions: Option<Tensor>,
}
