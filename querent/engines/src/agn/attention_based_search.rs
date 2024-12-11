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

use std::collections::HashSet;

/// Represents an entity with a name and its start and end indices in the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
	/// The name of the entity.
	pub name: String,
	/// The start index of the entity in the text.
	pub start_idx: usize,
	/// The end index of the entity in the text.
	pub end_idx: usize,
}

/// Represents a pair of entities (head and tail) within a context.
#[derive(Debug, Clone)]
pub struct EntityPair {
	/// The head entity.
	pub head_entity: Entity,
	/// The tail entity.
	pub tail_entity: Entity,
	/// The context in which the entities appear.
	pub context: String,
}

/// Represents a contextual relationship search with a score and visited tokens.
#[derive(Debug, Clone)]
pub struct SearchContextualRelationship {
	/// The current token being processed.
	pub current_token: usize,
	/// The total score of the relationship path.
	pub total_score: f32,
	/// The tokens that have been visited in this path.
	pub visited_tokens: Vec<usize>,
	/// The tokens that form the relationship.
	pub relation_tokens: Vec<usize>,
}

impl SearchContextualRelationship {
	/// Creates a new `SearchContextualRelationship` starting from the given initial token.
	pub fn new(initial_token_id: usize) -> Self {
		Self {
			current_token: initial_token_id,
			total_score: 0.0,
			visited_tokens: vec![initial_token_id],
			relation_tokens: Vec::new(),
		}
	}

	/// Adds a token to the relationship path, updating the score and visited tokens.
	///
	/// # Arguments
	///
	/// * `token_id` - The ID of the token to add.
	/// * `score` - The score associated with the token.
	pub fn add_token(&mut self, token_id: usize, score: f32) {
		self.current_token = token_id;
		self.visited_tokens.push(token_id);
		self.total_score += score;
		self.relation_tokens.push(token_id);
	}

	/// Checks if the relationship has any tokens.
	pub fn has_relation(&self) -> bool {
		!self.relation_tokens.is_empty()
	}

	/// Finalizes the relationship path by adding the given score.
	///
	/// # Arguments
	///
	/// * `score` - The score to add.
	pub fn finalize_path(&mut self, score: f32) {
		self.total_score += score;
	}

	/// Computes the mean score of the relationship path.
	pub fn mean_score(&self) -> f32 {
		if self.relation_tokens.is_empty() {
			0.0
		} else {
			self.total_score / self.relation_tokens.len() as f32
		}
	}
}

/// Sorts search contextual relationships by their mean score.
///
/// # Arguments
///
/// * `path` - The relationship path to compute the mean score for.
///
/// # Returns
///
/// The mean score of the relationship path.
pub fn sort_by_mean_score(path: &SearchContextualRelationship) -> f32 {
	path.mean_score()
}

/// Checks if a token is valid for inclusion in a relationship path.
///
/// # Arguments
///
/// * `token_id` - The ID of the token to check.
/// * `pair` - The entity pair being considered.
/// * `candidate_paths` - The current list of candidate paths.
/// * `current_path` - The current relationship path.
/// * `score` - The score associated with the token.
///
/// # Returns
///
/// A boolean indicating whether the token is valid for inclusion.
pub fn is_valid_token(
	token_id: usize,
	pair: &EntityPair,
	_candidate_paths: &mut Vec<SearchContextualRelationship>,
	_current_path: &mut SearchContextualRelationship,
	_score: f32,
) -> bool {
	!(pair.head_entity.start_idx..=pair.head_entity.end_idx).contains(&token_id) &&
		!(pair.tail_entity.start_idx..=pair.tail_entity.end_idx).contains(&token_id)
}

/// Performs a search for contextual relationships between entities using an attention matrix.
///
/// # Arguments
///
/// * `entity_start_index` - The starting index of the entity in the attention matrix.
/// * `attention_matrix` - The attention matrix used to compute scores between tokens.
/// * `entity_pair` - The pair of entities to find relationships for.
/// * `search_candidates` - The number of search candidates to consider.
/// * `require_contiguous` - Whether the tokens in the relationship must be contiguous.
/// * `max_relation_length` - The maximum length of the relationship in tokens.
///
/// # Returns
///
/// A result containing a vector of `SearchContextualRelationship` or an error.
pub fn perform_search(
	entity_start_index: usize,
	attention_matrix: &[Vec<f32>],
	entity_pair: &EntityPair,
	search_candidates: usize,
	require_contiguous: bool,
	max_relation_length: usize,
) -> Result<Vec<SearchContextualRelationship>, Box<dyn std::error::Error>> {
	let mut queue = vec![SearchContextualRelationship::new(entity_start_index)];
	let mut candidate_paths = Vec::new();
	let mut visited_paths = HashSet::new();

	while !queue.is_empty() {
		let mut current_path = queue.remove(0);

		if current_path.relation_tokens.len() > max_relation_length {
			continue;
		}

		if require_contiguous &&
			current_path.relation_tokens.len() > 1 &&
			(current_path.relation_tokens[current_path.relation_tokens.len() - 2] as isize -
				current_path.relation_tokens[current_path.relation_tokens.len() - 1] as isize)
				.abs() != 1
		{
			continue;
		}
		let mut new_paths = Vec::new();
		let attention_scores = &attention_matrix[current_path.current_token];

		for (i, &score) in attention_scores.iter().enumerate() {
			let mut next_path = current_path.visited_tokens.clone();
			next_path.push(i);

			if is_valid_token(i, entity_pair, &mut candidate_paths, &mut current_path, score) &&
				!visited_paths.contains(&next_path) &&
				current_path.current_token != i
			{
				let mut new_path = current_path.clone();
				new_path.add_token(i, attention_scores[i]);
				visited_paths.insert(next_path);
				new_paths.push(new_path);
			}
		}

		new_paths.sort_by(|a, b| b.mean_score().partial_cmp(&a.mean_score()).unwrap());
		queue.extend(new_paths.iter().take(search_candidates).cloned());
		candidate_paths.extend(new_paths.into_iter().take(search_candidates));
	}
	Ok(candidate_paths)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::utils::{ClassifiedSentenceWithAttention, ClassifiedSentenceWithPairs};
	#[test]
	fn test_perform_search() {
		// Define the classified sentence with attention
		let classified_sentence_with_attention = ClassifiedSentenceWithAttention {
			classified_sentence: ClassifiedSentenceWithPairs {
				sentence: "Joel lives in India and works for the company Microsoft.".to_string(),
				entities: vec![
					("joel".to_string(), "Unlabelled".to_string(), 0, 0),
					("india".to_string(), "Unlabelled".to_string(), 3, 3),
				],
				pairs: vec![("joel".to_string(), 0, 0, "india".to_string(), 3, 3)],
			},
			attention_matrix: Some(vec![
				vec![
					0.13502377,
					0.012803437,
					0.011252308,
					0.015802609,
					0.027309928,
					0.009818904,
					0.012140005,
					0.025747687,
					0.0043754,
					0.020907244,
					0.0295431,
				],
				vec![
					0.13478802,
					0.066507995,
					0.020229138,
					0.026512176,
					0.025660124,
					0.011827423,
					0.011404252,
					0.025268737,
					0.004008913,
					0.037393562,
					0.030990984,
				],
				vec![
					0.13898054,
					0.09479028,
					0.042532742,
					0.08783322,
					0.03801919,
					0.030534245,
					0.021959677,
					0.038614187,
					0.012880332,
					0.09460007,
					0.044915613,
				],
				vec![
					0.09276669,
					0.035715688,
					0.0424971,
					0.10774943,
					0.029270511,
					0.011864416,
					0.013519686,
					0.031020513,
					0.0042336946,
					0.052985482,
					0.0329762,
				],
				vec![
					0.20369516,
					0.035270546,
					0.025600951,
					0.03861624,
					0.044166528,
					0.021856476,
					0.021034244,
					0.050156906,
					0.011540355,
					0.107110314,
					0.05425207,
				],
				vec![
					0.11678322,
					0.06576122,
					0.037255622,
					0.030063821,
					0.038573496,
					0.029480545,
					0.023706809,
					0.043959837,
					0.019370556,
					0.12732796,
					0.050349392,
				],
				vec![
					0.18498766,
					0.06920747,
					0.032483988,
					0.04456716,
					0.042830132,
					0.037516907,
					0.02314554,
					0.04599428,
					0.01437487,
					0.1060448,
					0.05249863,
				],
				vec![
					0.1758304,
					0.03140701,
					0.026071362,
					0.038915604,
					0.04276128,
					0.022609502,
					0.021729475,
					0.05100949,
					0.012336625,
					0.12250277,
					0.054477,
				],
				vec![
					0.16627596,
					0.07487679,
					0.029465174,
					0.049319558,
					0.027568977,
					0.040357854,
					0.022713376,
					0.035723224,
					0.02342438,
					0.15214828,
					0.03951955,
				],
				vec![
					0.099376574,
					0.011397831,
					0.012472602,
					0.020036485,
					0.029697232,
					0.0074168434,
					0.010215441,
					0.037510116,
					0.0054607852,
					0.103350274,
					0.041891687,
				],
				vec![
					0.16987151,
					0.027351653,
					0.021941576,
					0.03672465,
					0.041878443,
					0.017951945,
					0.017710153,
					0.049282126,
					0.009208344,
					0.111788794,
					0.053433176,
				],
			]),
		};

		// Define the entity pair
		let entity_pair = EntityPair {
			head_entity: Entity { name: "joel".to_string(), start_idx: 0, end_idx: 0 },
			tail_entity: Entity { name: "india".to_string(), start_idx: 3, end_idx: 3 },
			context: "Joel lives in India and works in Microsoft.".to_string(),
		};

		// Get the attention matrix from the classified sentence with attention
		let attention_matrix = classified_sentence_with_attention.attention_matrix.unwrap();

		// Perform search
		let result = perform_search(
			0, // entity_start_index
			&attention_matrix,
			&entity_pair,
			2,    // search_candidates
			true, // require_contiguous
			2,    // max_relation_length
		);

		match result {
			Ok(paths) => {
				assert!(!paths.is_empty(), "No paths found");
			},
			Err(e) => tracing::error!("Error: {}", e),
		}
	}
}
