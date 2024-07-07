use std::collections::HashSet;

/// Represents an entity with a name and its start and end indices in the text.
#[derive(Debug, Clone)]
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
	candidate_paths: &mut Vec<SearchContextualRelationship>,
	current_path: &mut SearchContextualRelationship,
	score: f32,
) -> bool {
	if (pair.tail_entity.start_idx..=pair.tail_entity.end_idx).contains(&token_id) {
		if current_path.has_relation() {
			current_path.finalize_path(score);
			candidate_paths.push(current_path.clone());
			return false;
		}
	}

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

		// Get attention scores for the current token
		let attention_scores = &attention_matrix[current_path.current_token];

		for i in 0..attention_scores.len() {
			let next_path: Vec<_> =
				current_path.visited_tokens.iter().chain(std::iter::once(&i)).cloned().collect();
			if is_valid_token(
				i,
				entity_pair,
				&mut candidate_paths,
				&mut current_path,
				attention_scores[i],
			) && !visited_paths.contains(&next_path) &&
				current_path.current_token != i
			{
				let mut new_path = current_path.clone();
				new_path.add_token(i, attention_scores[i]);
				visited_paths.insert(next_path.clone());
				new_paths.push(new_path);
			}
		}

		new_paths.sort_by(|a, b| b.mean_score().partial_cmp(&a.mean_score()).unwrap());
		queue.extend(new_paths.into_iter().take(search_candidates));
	}
	Ok(candidate_paths)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::utils::{
//         ClassifiedSentenceWithAttention, ClassifiedSentenceWithPairs
//     };
//     #[test]
//     fn test_perform_search() {
//         // Define the classified sentence with attention
//         let classified_sentence_with_attention = ClassifiedSentenceWithAttention {
//             classified_sentence: ClassifiedSentenceWithPairs {
//                 sentence: "Joel 1 2 India.".to_string(),
//                 entities: vec![
//                     ("joel".to_string(), "Unlabelled".to_string(), 0, 0),
//                     ("india".to_string(), "Unlabelled".to_string(), 3, 3),
//                 ],
//                 pairs: vec![("joel".to_string(), 0, 0, "india".to_string(), 3, 3)],
//             },
//             attention_matrix: Some(vec![
//                 vec![0.17606843, 0.027115423, 0.02369972, 0.035531946, 0.040166296],
//                 vec![0.1329437, 0.095136136, 0.035234112, 0.03919317, 0.05276324],
//                 vec![0.2027459, 0.10026775, 0.056107037, 0.09195952, 0.07401152],
//                 vec![0.097249776, 0.04050471, 0.038452335, 0.12229665, 0.04973845],
//                 vec![0.22857486, 0.06029765, 0.04682016, 0.06759972, 0.07255538],
//             ]),
//         };

//         // Define the entity pair
//         let entity_pair = EntityPair {
//             head_entity: Entity {
//                 name: "joel".to_string(),
//                 start_idx: 0,
//                 end_idx: 0,
//             },
//             tail_entity: Entity {
//                 name: "india".to_string(),
//                 start_idx: 3,
//                 end_idx: 3,
//             },
//             context: "Joel lives in India.".to_string(),
//         };

//         // Get the attention matrix from the classified sentence with attention
//         let attention_matrix = classified_sentence_with_attention.attention_matrix.unwrap();

//         // Perform search
//         let result = perform_search(
//             0, // entity_start_index
//             &attention_matrix,
//             &entity_pair,
//             2, // search_candidates
//             true, // require_contiguous
//             2, // max_relation_length
//         );

//         match result {
//             Ok(paths) => {
//                 println!("Found paths: {:?}", paths);
//                 assert!(!paths.is_empty(), "No paths found");
//             },
//             Err(e) => println!("Error: {}", e),
//         }
//     }
// }
