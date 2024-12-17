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

use crate::agn::attention_based_search::Entity;
use std::collections::HashSet;

/// Represents the relationship between a head and a tail entity, including associated relations and their scores.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadTailRelations {
	/// The head entity in the relationship.
	pub head: Entity,
	/// The tail entity in the relationship.
	pub tail: Entity,
	/// A vector of tuples where each tuple contains a relation string and its corresponding score.
	pub relations: Vec<(String, f32)>,
}

/// Represents a search beam used in the filtering process.
#[derive(Debug, Clone)]
pub struct SearchBeam {
	/// A vector of token indices representing the relation tokens.
	pub rel_tokens: Vec<usize>,
	/// The score of the search beam.
	pub score: f32,
}

impl SearchBeam {
	/// Computes the mean score of the search beam by dividing the total score by the number of relation tokens.
	pub fn mean_score(&self) -> f32 {
		self.score / self.rel_tokens.len() as f32
	}
}

/// Represents a token with its text and lemma.
#[derive(Debug, Clone)]
pub struct Token {
	/// The text of the token.
	pub text: String,
	/// The lemma of the token.
	pub lemma: String,
}

/// A filter used to filter search beams based on document tokens, forward relations, and a score threshold.
pub struct IndividualFilter {
	/// A vector of tokens representing the document.
	doc: Vec<Token>,
	/// A flag indicating whether to use forward relations.
	forward_relations: bool,
	/// The threshold score for filtering search beams.
	threshold: f32,
}

impl IndividualFilter {
	/// Creates a new `IndividualFilter` with the specified document tokens, forward relations flag, and threshold score.
	pub fn new(doc: Vec<Token>, forward_relations: bool, threshold: f32) -> Self {
		Self { doc, forward_relations, threshold }
	}

	/// Filters the given candidates and returns the head-tail relations that meet the criteria.
	///
	/// # Arguments
	///
	/// * `candidates` - A vector of search beams to filter.
	/// * `head` - The head entity.
	/// * `tail` - The tail entity.
	///
	/// # Returns
	///
	/// A `HeadTailRelations` struct containing the filtered relations.
	pub fn filter(
		&self,
		candidates: Vec<SearchBeam>,
		head: &Entity,
		tail: &Entity,
	) -> HeadTailRelations {
		let mut response =
			HeadTailRelations { head: head.clone(), tail: tail.clone(), relations: Vec::new() };

		let mut seen_relations = HashSet::new();

		for candidate in candidates {
			if candidate.score < self.threshold {
				continue;
			}

			let mut rel_txt = String::new();
			let mut last_index = None;
			let mut valid = true;

			for &token_id in &candidate.rel_tokens {
				let word_id = token_id;
				let word = &self.doc[token_id].text;
				if self.forward_relations {
					if let Some(last_idx) = last_index {
						if token_id <= last_idx || token_id - last_idx != 1 {
							valid = false;
							break;
						}
					}
				}
				last_index = Some(word_id);

				if !rel_txt.is_empty() {
					rel_txt.push(' ');
				}
				let lowered_word = word.to_lowercase();
				if !head.name.eq_ignore_ascii_case(&lowered_word) &&
					!tail.name.eq_ignore_ascii_case(&lowered_word)
				{
					rel_txt.push_str(&lowered_word);
				}
			}
			if valid {
				let lemmatized_txt = self.simple_filter(&rel_txt);
				if !lemmatized_txt.is_empty() && seen_relations.insert(lemmatized_txt.clone()) {
					response.relations.push((lemmatized_txt, candidate.score));
				}
			}
		}

		response
	}

	/// Applies a simple filter to the relation string, removing non-alphanumeric words.
	///
	/// # Arguments
	///
	/// * `relation` - The relation string to filter.
	///
	/// # Returns
	///
	/// A filtered relation string containing only alphanumeric words.
	fn simple_filter(&self, relation: &str) -> String {
		let filtered_relation = relation
			.split_whitespace()
			.filter(|word| word.chars().all(char::is_alphanumeric))
			.collect::<Vec<&str>>()
			.join(" ");
		filtered_relation
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_individual_filter() {
		let doc = vec![
			Token { text: "Joel".to_string(), lemma: "joel".to_string() },
			Token { text: "lives".to_string(), lemma: "live".to_string() },
			Token { text: "in".to_string(), lemma: "in".to_string() },
			Token { text: "India".to_string(), lemma: "india".to_string() },
			Token { text: "and".to_string(), lemma: "and".to_string() },
			Token { text: "works".to_string(), lemma: "work".to_string() },
			Token { text: "for".to_string(), lemma: "for".to_string() },
			Token { text: "the".to_string(), lemma: "the".to_string() },
			Token { text: "company".to_string(), lemma: "company".to_string() },
			Token { text: "Microsoft".to_string(), lemma: "microsoft".to_string() },
			Token { text: ".".to_string(), lemma: ".".to_string() },
		];

		let filter = IndividualFilter::new(doc, true, 0.01);

		let candidates = vec![
			SearchBeam { rel_tokens: vec![10], score: 0.0295431 },
			SearchBeam { rel_tokens: vec![4], score: 0.027309928 },
			SearchBeam { rel_tokens: vec![10, 9], score: 0.1413319 },
			SearchBeam { rel_tokens: vec![10, 7], score: 0.07882523 },
			SearchBeam { rel_tokens: vec![4, 9], score: 0.13442025 },
			SearchBeam { rel_tokens: vec![4, 10], score: 0.081562 },
			SearchBeam { rel_tokens: vec![10, 9, 10], score: 0.18322358 },
			SearchBeam { rel_tokens: vec![10, 9, 7], score: 0.17884201 },
		];

		let head = Entity { name: "joel".to_string(), start_idx: 0, end_idx: 0 };
		let tail = Entity { name: "india".to_string(), start_idx: 3, end_idx: 3 };

		let result = filter.filter(candidates, &head, &tail);
		assert_eq!(result.relations.len(), 1);
	}
}
