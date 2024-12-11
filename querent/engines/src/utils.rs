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

use std::{collections::HashMap, sync::Arc};

use crate::{agn::HeadTailRelations, EngineError, EngineErrorKind};
use chrono::{TimeZone, Utc};
use fastembed::TextEmbedding;
use lazy_static::lazy_static;
use llms::LLM;
use rand::{thread_rng, Rng};
use regex::Regex;
use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;
/// Represents a classified sentence with identified entities.
#[derive(Debug, Serialize, PartialEq)]
pub struct ClassifiedSentence {
	/// The classified sentence as a string.
	pub sentence: String,
	/// A vector of tuples representing entities in the sentence. Each tuple contains:
	/// (entity text, entity label, start index, end index).
	pub entities: Vec<(String, String, usize, usize)>,
}

/// Represents a classified sentence with identified entity pairs.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ClassifiedSentenceWithPairs {
	/// The classified sentence as a string.
	pub sentence: String,
	/// A vector of tuples representing entities in the sentence. Each tuple contains:
	/// (entity text, entity label, start index, end index).
	pub entities: Vec<(String, String, usize, usize)>,
	/// A vector of tuples representing pairs of entities in the sentence. Each tuple contains:
	/// (entity1 text, start index of entity1, end index of entity1, entity2 text, start index of entity2, end index of entity2).
	pub pairs: Vec<(String, usize, usize, String, usize, usize)>,
}

/// Represents a classified sentence with attention matrix.
#[derive(Debug, Serialize, Clone)]
pub struct ClassifiedSentenceWithAttention {
	/// The classified sentence with pairs.
	pub classified_sentence: ClassifiedSentenceWithPairs,
	/// The attention matrix for the sentence (optional).
	pub attention_matrix: Option<Vec<Vec<f32>>>,
}

/// Represents a classified sentence with identified relations and attention matrix.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassifiedSentenceWithRelations {
	/// The classified sentence with pairs.
	pub classified_sentence: ClassifiedSentenceWithPairs,
	/// The attention matrix for the sentence (optional).
	pub attention_matrix: Option<Vec<Vec<f32>>>,
	/// A vector of head-tail relations found in the sentence.
	pub relations: Vec<HeadTailRelations>,
}

lazy_static! {
	static ref NEWLINE_RE: Regex = Regex::new(r"\n+").unwrap();
}
pub fn remove_newlines(text: &str) -> String {
	let sanitized_text = sanitize_text(text);
	NEWLINE_RE.replace_all(&sanitized_text, " ").to_string()
}

/// Removes null bytes and any other invalid UTF-8 sequences from the given text.
fn sanitize_text(input: &str) -> String {
	input.chars().filter(|&c| c != '\0').collect()
}

/// Splits the provided text into a vector of sentences.
pub fn split_into_sentences(text: &str) -> Vec<String> {
	let repeated_pattern = match Regex::new(r"[^a-zA-Z0-9]{5,}") {
		Ok(re) => re,
		Err(e) => {
			eprintln!("Error creating regex: {}. Using default pattern.", e);
			Regex::new(r"\.{5,}").expect("Default regex should never fail")
		},
	};
	UnicodeSegmentation::split_sentence_bounds(text)
		.map(|sentence| sentence.trim())
		.filter(|s| !s.is_empty())
		.map(|s| repeated_pattern.replace_all(s, " ").to_string())
		.filter(|s| !s.is_empty())
		.map(String::from)
		.collect()
}

/// Splits the provided text into chunks based on the maximum token length.
pub fn split_into_chunks(max_tokens: usize, tokens: &str) -> Vec<String> {
	let sentences = split_into_sentences(tokens);
	let mut chunks = Vec::new();
	let mut current_chunk = String::new();
	let mut current_length = 0;

	for sentence in sentences {
		let sentence_length = sentence.chars().count();

		if current_length + sentence_length > max_tokens {
			if !current_chunk.is_empty() {
				chunks.push(current_chunk.clone());
				current_chunk.clear();
				current_length = 0;
			}

			if sentence_length > max_tokens {
				let mut start = 0;
				let sentence_chars: Vec<char> = sentence.chars().collect();
				while start < sentence_length {
					let end = std::cmp::min(start + max_tokens, sentence_length);
					let chunk: String = sentence_chars[start..end].iter().collect();
					chunks.push(chunk);
					start = end;
				}
			} else {
				current_chunk = sentence;
				current_length = sentence_length;
			}
		} else {
			if !current_chunk.is_empty() {
				current_chunk.push(' ');
			}
			current_chunk.push_str(&sentence);
			current_length += sentence_length;
		}
	}

	if !current_chunk.is_empty() {
		chunks.push(current_chunk);
	}

	chunks
}

/// Labels entities in the provided sentences based on the list of entities.
pub fn label_entities_in_sentences(
	entities: &[String],
	sentences: &[String],
) -> Vec<ClassifiedSentence> {
	let mut classified_sentences = Vec::new();
	for sentence in sentences {
		let mut found_entities = Vec::new();
		for entity in entities {
			let positions = find_entity_indices(sentence, entity);
			for (start, end) in positions {
				found_entities.push((entity.clone(), "Unlabelled".to_string(), start, end));
			}
		}
		classified_sentences
			.push(ClassifiedSentence { sentence: sentence.clone(), entities: found_entities });
	}
	classified_sentences
}

/// Finds the start and end indices of the given entity in the sentence.
fn find_entity_indices(sentence: &str, entity: &str) -> Vec<(usize, usize)> {
	let sentence_lower = sentence.to_lowercase();
	let entity_lower = entity.to_lowercase();
	let mut positions = Vec::new();
	let mut start_pos = 0;
	while let Some(pos) = sentence_lower[start_pos..].find(&entity_lower) {
		let start = start_pos + pos;
		let end = start + entity.len();
		positions.push((start, end));
		start_pos = end;
	}
	positions
}

/// Converts tokens to words using the provided LLM.
pub async fn tokens_to_words(llm: &dyn LLM, tokens: &[Vec<i32>]) -> Vec<Vec<String>> {
	let mut words_list = Vec::new();
	for token_seq in tokens {
		let words = llm.tokens_to_words(token_seq).await;
		words_list.push(words);
	}
	words_list
}

/// Matches entities with tokens in the provided sentences.
pub fn match_entities_with_tokens(
	tokenized_sentences: &[Vec<String>],
	classified_sentences: &[ClassifiedSentence],
) -> Vec<ClassifiedSentence> {
	let mut results = Vec::new();

	for (sentence_index, classified_sentence) in classified_sentences.iter().enumerate() {
		let sentence_tokens = &tokenized_sentences[sentence_index];
		let mut matched_entities = Vec::new();
		let mut seen_positions = std::collections::HashSet::new();

		for (entity, label, _start, _end) in &classified_sentence.entities {
			let token_indices = find_all_token_indices(sentence_tokens, entity);
			for (token_start, token_end) in token_indices {
				if seen_positions.insert((token_start, token_end)) {
					matched_entities.push((
						entity.to_string(),
						label.to_string(),
						token_start,
						token_end,
					));
				}
			}
		}

		results.push(ClassifiedSentence {
			sentence: classified_sentence.sentence.clone(),
			entities: matched_entities,
		});
	}

	results
}

/// Finds all token indices for the given entity in the token list.
fn find_all_token_indices(tokens: &[String], entity: &str) -> Vec<(usize, usize)> {
	let mut indices = Vec::new();
	if entity.is_empty() {
		return indices;
	}

	let entity_tokens: Vec<String> = entity.split_whitespace().map(|e| e.to_lowercase()).collect();
	let token_count = tokens.len();
	let entity_token_count = entity_tokens.len();

	// Ensure that the entity has at least one token and that the tokens list is long enough
	if entity_token_count == 0 || token_count == 0 || entity_token_count > token_count {
		return indices;
	}

	// Convert all tokens to lowercase once
	let tokens_lower: Vec<String> = tokens.iter().map(|t| t.to_lowercase()).collect();

	for i in 0..=token_count.saturating_sub(entity_token_count) {
		// Compare token slices directly without allocating a new Vec each time
		if tokens_lower[i..i + entity_token_count] == entity_tokens[..] {
			indices.push((i, i + entity_token_count - 1));
		}
	}

	indices
}

/// Creates binary pairs of entities in the provided sentences.
pub fn create_binary_pairs(
	classified_sentences: &[ClassifiedSentence],
) -> Vec<ClassifiedSentenceWithPairs> {
	let mut results = Vec::new();

	for classified_sentence in classified_sentences {
		let sentence = &classified_sentence.sentence;
		let mut entities = classified_sentence.entities.clone();

		// Sort entities by their start indices
		entities.sort_by_key(|k| k.2);

		if entities.len() > 1 {
			let mut pairs = Vec::new();

			for i in 0..entities.len() {
				for j in (i + 1)..entities.len() {
					let (entity1, _, start1, end1) = &entities[i];
					let (entity2, _, start2, end2) = &entities[j];

					// Skip if the entities are the same or if either is [UNK]
					if entity1 == entity2 || entity1 == "[UNK]" || entity2 == "[UNK]" {
						continue;
					}

					// Add constraint: entity start index should not be between another entity's start and end indices
					if (*start1 >= *start2 && *start1 <= *end2) ||
						(*start2 >= *start1 && *start2 <= *end1)
					{
						continue;
					}

					// Calculate the character distance between entity1 and entity2
					let char_distance = (*start1 as isize - *start2 as isize).abs();

					// Skip pairs where the distance is greater than 25 characters
					if char_distance > 25 {
						continue;
					}

					pairs.push((
						entity1.to_string(),
						*start1,
						*end1,
						entity2.to_string(),
						*start2,
						*end2,
					));
				}
			}

			results.push(ClassifiedSentenceWithPairs {
				sentence: sentence.to_string(),
				entities,
				pairs,
			});
		} else {
			results.push(ClassifiedSentenceWithPairs {
				sentence: sentence.to_string(),
				entities,
				pairs: Vec::new(),
			});
		}
	}

	results
}

/// Selects the relationship with the highest score for an entity pair
pub fn select_highest_score_relation(head_tail_relations: &HeadTailRelations) -> HeadTailRelations {
	if head_tail_relations.relations.is_empty() {
		return head_tail_relations.clone();
	}
	let highest_relation = head_tail_relations
		.relations
		.iter()
		.max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

	// Handle the case where no relation is found
	match highest_relation {
		Some(relation) => HeadTailRelations {
			head: head_tail_relations.head.clone(),
			tail: head_tail_relations.tail.clone(),
			relations: vec![relation.clone()],
		},
		None => head_tail_relations.clone(),
	}
}

/// Adds attention matrices to the classified sentences with pairs using the provided LLM.
pub async fn add_attention_to_classified_sentences(
	llm: &dyn LLM,
	classified_sentences_with_pairs: &[ClassifiedSentenceWithPairs],
	tokenized_chunks: &[Vec<i32>],
) -> Result<Vec<ClassifiedSentenceWithAttention>, EngineError> {
	let mut extended_classified_sentences_with_attention = Vec::new();

	for (index, classified_sentence) in classified_sentences_with_pairs.iter().enumerate() {
		if !classified_sentence.pairs.is_empty() {
			// Get the corresponding tokenized chunk
			let tokenized_chunk = &tokenized_chunks[index];

			// Prepare model input for inference attention
			let model_input =
				llm.model_input(tokenized_chunk.clone()).await.map_err(EngineError::from)?;

			// Perform inference attention
			let attention_result =
				llm.inference_attention(model_input).await.map_err(EngineError::from)?;

			// Convert the attention tensor to a 2D vector
			let attention_weights = llm
				.attention_tensor_to_2d_vector(&attention_result)
				.await
				.map_err(EngineError::from)?;

			// Add the attention matrix to the classified sentence
			extended_classified_sentences_with_attention.push(ClassifiedSentenceWithAttention {
				classified_sentence: classified_sentence.clone(),
				attention_matrix: Some(attention_weights),
			});
		} else {
			// If no entity pairs, add the classified sentence with an empty 2D vector
			let empty_attention_weights = vec![];
			extended_classified_sentences_with_attention.push(ClassifiedSentenceWithAttention {
				classified_sentence: classified_sentence.clone(),
				attention_matrix: Some(empty_attention_weights),
			});
		}
	}

	Ok(extended_classified_sentences_with_attention)
}

/// Merges similar relations in the provided sentences with relations.
pub fn merge_similar_relations(sentences_with_relations: &mut [ClassifiedSentenceWithRelations]) {
	for sentence in sentences_with_relations {
		for relation in &mut sentence.relations {
			let mut merged_relations: HashMap<String, f32> = HashMap::new();

			for (rel, score) in &relation.relations {
				let mut _merged = false;
				let mut keys_to_remove = Vec::new();
				let mut new_key = rel.clone();
				let mut new_score = *score;

				for (existing_rel, existing_score) in &mut merged_relations {
					if existing_rel.contains(rel) || rel.contains(existing_rel) {
						new_key = if rel.len() > existing_rel.len() {
							rel.clone()
						} else {
							existing_rel.clone()
						};
						new_score += *existing_score;
						keys_to_remove.push(existing_rel.clone());
						_merged = true;
					}
				}

				for key in keys_to_remove {
					merged_relations.remove(&key);
				}

				merged_relations.insert(new_key, new_score);
			}

			relation.relations = merged_relations.into_iter().collect();
		}
	}
}

/// Utility function to calculate biased sentence embedding.
pub async fn calculate_biased_sentence_embedding(
	embedder: &TextEmbedding,
	attention_matrix: &Vec<Vec<f32>>,
	head_entity: &str,
	tail_entity: &str,
	predicate: &str,
	score: &f32,
	sentence: &str,
	head_start_idx: usize,
	head_end_idx: usize,
	tail_start_idx: usize,
	tail_end_idx: usize,
) -> Result<Vec<f32>, EngineError> {
	// Obtain embeddings
	let sentence_embedding = embedder.embed(vec![sentence.to_string()], None).map_err(|e| {
		EngineError::new(EngineErrorKind::ModelError, Arc::new(anyhow::anyhow!(e.to_string())))
	})?[0]
		.clone();
	let head_embedding = embedder.embed(vec![head_entity.to_string()], None).map_err(|e| {
		EngineError::new(EngineErrorKind::ModelError, Arc::new(anyhow::anyhow!(e.to_string())))
	})?[0]
		.clone();
	let tail_embedding = embedder.embed(vec![tail_entity.to_string()], None).map_err(|e| {
		EngineError::new(EngineErrorKind::ModelError, Arc::new(anyhow::anyhow!(e.to_string())))
	})?[0]
		.clone();
	let predicate_embedding = embedder.embed(vec![predicate.to_string()], None).map_err(|e| {
		EngineError::new(EngineErrorKind::ModelError, Arc::new(anyhow::anyhow!(e.to_string())))
	})?[0]
		.clone();

	// Calculate attention scores for head and tail entities
	let head_attention_score: f32 = attention_matrix
		.iter()
		.map(|row| {
			(head_start_idx..=head_end_idx).map(|idx| row[idx]).sum::<f32>() /
				(head_end_idx - head_start_idx + 1) as f32
		})
		.sum::<f32>() /
		attention_matrix.len() as f32;

	let tail_attention_score: f32 = attention_matrix
		.iter()
		.map(|row| {
			(tail_start_idx..=tail_end_idx).map(|idx| row[idx]).sum::<f32>() /
				(tail_end_idx - tail_start_idx + 1) as f32
		})
		.sum::<f32>() /
		attention_matrix.len() as f32;
	let mut biased_sentence_embedding = sentence_embedding;
	for i in 0..biased_sentence_embedding.len() {
		biased_sentence_embedding[i] += head_attention_score * head_embedding[i] +
			tail_attention_score * tail_embedding[i] +
			score * predicate_embedding[i];
	}
	let norm: f32 = biased_sentence_embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
	let normalized_biased_sentence_embedding: Vec<f32> =
		biased_sentence_embedding.iter().map(|&x| x / norm).collect();

	Ok(normalized_biased_sentence_embedding)
}

/// Generates a custom UUID based on the current time and a random number.
pub fn generate_custom_comb_uuid() -> String {
	let custom_epoch = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
	let now = Utc::now();
	let millis_since_epoch = now.signed_duration_since(custom_epoch).num_milliseconds();
	let timestamp_part = (millis_since_epoch as u64) & 0x000F_FFFF_FFFF_FFFF;
	let mut rng = thread_rng();
	let random_part: u16 = rng.gen_range(0..4096);
	let uuid_int = (timestamp_part << 12) | (random_part as u64);

	uuid_int.to_string()
}

/// Extracts entities and their types from classified sentences with relations.
pub fn extract_entities_and_types(
	all_sentences_with_relations: Vec<ClassifiedSentenceWithRelations>,
) -> (Vec<String>, Vec<String>) {
	let mut entities = Vec::new();
	let mut sample_entities = Vec::new();

	for item in all_sentences_with_relations {
		for (entity, entity_type, _start_idx, _end_idx) in item.classified_sentence.entities.clone()
		{
			entities.push(entity);
			sample_entities.push(entity_type);
		}
	}

	(entities, sample_entities)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		agn::Entity,
		utils::{ClassifiedSentence, ClassifiedSentenceWithPairs},
	};

	#[test]
	fn test_remove_newlines_special_characters() {
		let input = "Special chars:\n\n\t\n\rTest.";
		let expected = "Special chars: \t \rTest.";
		assert_eq!(remove_newlines(input), expected);
	}

	#[test]
	fn test_split_into_sentences_special() {
		let input = "Wait... What?! No way!!!";
		let expected = vec!["Wait...", "What?!", "No way!!!"];
		assert_eq!(split_into_sentences(input), expected);
	}

	#[test]
	fn test_split_into_chunks_special_long_sentence() {
		let input = "A".repeat(50);
		let max_tokens = 10;
		let expected = vec!["AAAAAAAAAA", "AAAAAAAAAA", "AAAAAAAAAA", "AAAAAAAAAA", "AAAAAAAAAA"];
		assert_eq!(split_into_chunks(max_tokens, &input), expected);
	}

	#[test]
	fn test_label_entities_in_sentences_positive() {
		let entities = vec!["test".to_string(), "sentence".to_string()];
		let sentences = vec!["This is a test sentence.".to_string()];
		let expected = vec![ClassifiedSentence {
			sentence: "This is a test sentence.".to_string(),
			entities: vec![
				("test".to_string(), "Unlabelled".to_string(), 10, 14),
				("sentence".to_string(), "Unlabelled".to_string(), 15, 23),
			],
		}];
		assert_eq!(label_entities_in_sentences(&entities, &sentences), expected);
	}

	#[test]
	fn test_label_entities_in_sentences_negative() {
		let entities = vec!["nonexistent".to_string()];
		let sentences = vec!["This sentence has no entities.".to_string()];
		let expected = vec![ClassifiedSentence {
			sentence: "This sentence has no entities.".to_string(),
			entities: vec![],
		}];
		assert_eq!(label_entities_in_sentences(&entities, &sentences), expected);
	}

	#[test]
	fn test_label_entities_in_sentences_special_overlapping_entities() {
		let entities = vec!["test".to_string(), "test sentence".to_string()];
		let sentences = vec!["This is a test sentence.".to_string()];
		let expected = vec![ClassifiedSentence {
			sentence: "This is a test sentence.".to_string(),
			entities: vec![
				("test".to_string(), "Unlabelled".to_string(), 10, 14),
				("test sentence".to_string(), "Unlabelled".to_string(), 10, 23),
			],
		}];
		assert_eq!(label_entities_in_sentences(&entities, &sentences), expected);
	}
	#[test]
	fn test_find_entity_indices_special_multiple_occurrences() {
		let sentence = "Test this test sentence with test cases.";
		let entity = "test";
		let expected = vec![(0, 4), (10, 14), (29, 33)];
		assert_eq!(find_entity_indices(sentence, entity), expected);
	}

	#[test]
	fn test_match_entities_with_tokens_positive() {
		let tokenized_sentences = vec![vec![
			"This".to_string(),
			"is".to_string(),
			"a".to_string(),
			"test".to_string(),
			"sentence.".to_string(),
		]];
		let classified_sentences = vec![ClassifiedSentence {
			sentence: "This is a test sentence.".to_string(),
			entities: vec![("test".to_string(), "Unlabelled".to_string(), 10, 14)],
		}];
		let expected = vec![ClassifiedSentence {
			sentence: "This is a test sentence.".to_string(),
			entities: vec![("test".to_string(), "Unlabelled".to_string(), 3, 3)],
		}];
		assert_eq!(
			match_entities_with_tokens(&tokenized_sentences, &classified_sentences),
			expected
		);
	}

	#[test]
	fn test_match_entities_with_tokens_negative() {
		let tokenized_sentences = vec![vec![
			"This".to_string(),
			"is".to_string(),
			"a".to_string(),
			"test".to_string(),
			"sentence.".to_string(),
		]];
		let classified_sentences = vec![ClassifiedSentence {
			sentence: "This is a test sentence.".to_string(),
			entities: vec![("nonexistent".to_string(), "Unlabelled".to_string(), 0, 10)],
		}];
		let expected = vec![ClassifiedSentence {
			sentence: "This is a test sentence.".to_string(),
			entities: vec![],
		}];
		assert_eq!(
			match_entities_with_tokens(&tokenized_sentences, &classified_sentences),
			expected
		);
	}

	#[test]
	fn test_match_entities_with_tokens_special_overlapping_entities() {
		let tokenized_sentences = vec![vec![
			"This".to_string(),
			"is".to_string(),
			"a".to_string(),
			"test".to_string(),
			"sentence".to_string(),
			".".to_string(),
		]];
		let classified_sentences = vec![ClassifiedSentence {
			sentence: "This is a test sentence.".to_string(),
			entities: vec![
				("test".to_string(), "Unlabelled".to_string(), 10, 14),
				("test sentence".to_string(), "Unlabelled".to_string(), 10, 23),
			],
		}];
		let expected = vec![ClassifiedSentence {
			sentence: "This is a test sentence.".to_string(),
			entities: vec![
				("test".to_string(), "Unlabelled".to_string(), 3, 3),
				("test sentence".to_string(), "Unlabelled".to_string(), 3, 4),
			],
		}];
		assert_eq!(
			match_entities_with_tokens(&tokenized_sentences, &classified_sentences),
			expected
		);
	}

	#[test]
	fn test_create_binary_pairs_positive() {
		let classified_sentences = vec![ClassifiedSentence {
			sentence: "Alice knows Bob.".to_string(),
			entities: vec![
				("Alice".to_string(), "Person".to_string(), 0, 5),
				("Bob".to_string(), "Person".to_string(), 12, 15),
			],
		}];
		let expected = vec![ClassifiedSentenceWithPairs {
			sentence: "Alice knows Bob.".to_string(),
			entities: vec![
				("Alice".to_string(), "Person".to_string(), 0, 5),
				("Bob".to_string(), "Person".to_string(), 12, 15),
			],
			pairs: vec![("Alice".to_string(), 0, 5, "Bob".to_string(), 12, 15)],
		}];
		assert_eq!(create_binary_pairs(&classified_sentences), expected);
	}

	#[test]
	fn test_create_binary_pairs_negative() {
		let classified_sentences = vec![ClassifiedSentence {
			sentence: "No entities here.".to_string(),
			entities: vec![],
		}];
		let expected = vec![ClassifiedSentenceWithPairs {
			sentence: "No entities here.".to_string(),
			entities: vec![],
			pairs: vec![],
		}];
		assert_eq!(create_binary_pairs(&classified_sentences), expected);
	}

	#[test]
	fn test_select_highest_score_relation_positive() {
		let input = HeadTailRelations {
			head: Entity { name: "Alice".to_string(), start_idx: 0, end_idx: 5 },
			tail: Entity { name: "Bob".to_string(), start_idx: 10, end_idx: 13 },
			relations: vec![("friend".to_string(), 0.8), ("colleague".to_string(), 0.5)],
		};
		let expected = HeadTailRelations {
			head: Entity { name: "Alice".to_string(), start_idx: 0, end_idx: 5 },
			tail: Entity { name: "Bob".to_string(), start_idx: 10, end_idx: 13 },
			relations: vec![("friend".to_string(), 0.8)],
		};
		assert_eq!(select_highest_score_relation(&input), expected);
	}

	#[test]
	fn test_select_highest_score_relation_special_same_score() {
		let mut input = HeadTailRelations {
			head: Entity { name: "Alice".to_string(), start_idx: 0, end_idx: 5 },
			tail: Entity { name: "Bob".to_string(), start_idx: 10, end_idx: 13 },
			relations: vec![("friend".to_string(), 0.8), ("colleague".to_string(), 0.8)],
		};
		input = select_highest_score_relation(&input);
		assert!(input.relations.len() == 1);
	}

	#[test]
	fn test_merge_similar_relations_positive() {
		let mut sentences_with_relations = vec![ClassifiedSentenceWithRelations {
			classified_sentence: ClassifiedSentenceWithPairs {
				sentence: "Alice and Bob are colleagues.".to_string(),
				entities: vec![
					("Alice".to_string(), "Person".to_string(), 0, 5),
					("Bob".to_string(), "Person".to_string(), 10, 13),
				],
				pairs: vec![],
			},
			attention_matrix: None,
			relations: vec![HeadTailRelations {
				head: Entity { name: "Alice".to_string(), start_idx: 0, end_idx: 5 },
				tail: Entity { name: "Bob".to_string(), start_idx: 10, end_idx: 13 },
				relations: vec![("colleague".to_string(), 0.7), ("colleagues".to_string(), 0.6)],
			}],
		}];

		merge_similar_relations(&mut sentences_with_relations);

		let expected = vec![ClassifiedSentenceWithRelations {
			classified_sentence: ClassifiedSentenceWithPairs {
				sentence: "Alice and Bob are colleagues.".to_string(),
				entities: vec![
					("Alice".to_string(), "Person".to_string(), 0, 5),
					("Bob".to_string(), "Person".to_string(), 10, 13),
				],
				pairs: vec![],
			},
			attention_matrix: None,
			relations: vec![HeadTailRelations {
				head: Entity { name: "Alice".to_string(), start_idx: 0, end_idx: 5 },
				tail: Entity { name: "Bob".to_string(), start_idx: 10, end_idx: 13 },
				relations: vec![("colleagues".to_string(), 1.3)],
			}],
		}];

		assert_eq!(sentences_with_relations, expected);
	}

	#[test]
	fn test_extract_entities_and_types_special_duplicate_entities() {
		let sentences_with_relations = vec![ClassifiedSentenceWithRelations {
			classified_sentence: ClassifiedSentenceWithPairs {
				sentence: "Alice knows Alice.".to_string(),
				entities: vec![
					("Alice".to_string(), "Person".to_string(), 0, 5),
					("Alice".to_string(), "Person".to_string(), 12, 17),
				],
				pairs: vec![],
			},
			attention_matrix: None,
			relations: vec![],
		}];
		let (entities, types) = extract_entities_and_types(sentences_with_relations);
		let expected_entities = vec!["Alice".to_string(), "Alice".to_string()];
		let expected_types = vec!["Person".to_string(), "Person".to_string()];
		assert_eq!(entities, expected_entities);
		assert_eq!(types, expected_types);
	}
}
