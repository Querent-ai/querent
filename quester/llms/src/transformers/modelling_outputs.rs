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
