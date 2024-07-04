use std::{collections::HashMap, sync::Arc};

use crate::{agn::HeadTailRelations, EngineError, EngineErrorKind};
use chrono::{TimeZone, Utc};
use fastembed::TextEmbedding;
use llms::LLM;
use rand::{thread_rng, Rng};
use regex::Regex;
use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Serialize)]
pub struct ClassifiedSentence {
	pub sentence: String,
	pub entities: Vec<(String, String, usize, usize)>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClassifiedSentenceWithPairs {
	pub sentence: String,
	pub entities: Vec<(String, String, usize, usize)>, // (entity, label, start, end)
	pub pairs: Vec<(String, usize, usize, String, usize, usize)>, // (entity1, start1, end1, entity2, start2, end2)
}

#[derive(Debug, Serialize, Clone)]
pub struct ClassifiedSentenceWithAttention {
	pub classified_sentence: ClassifiedSentenceWithPairs,
	pub attention_matrix: Option<Vec<Vec<f32>>>,
}

#[derive(Debug, Clone)]
pub struct ClassifiedSentenceWithRelations {
	pub classified_sentence: ClassifiedSentenceWithPairs,
	pub attention_matrix: Option<Vec<Vec<f32>>>,
	pub relations: Vec<HeadTailRelations>,
}

/// Removes newline characters from the given text.
pub fn remove_newlines(text: &str) -> String {
	let sanitized_text = sanitize_text(text);
	let re = Regex::new(r"\n+").unwrap();
	re.replace_all(&sanitized_text, " ").to_string()
}

/// Remove null bytes and any other invalid UTF-8 sequences
fn sanitize_text(input: &str) -> String {
	input.chars().filter(|&c| c != '\0').collect()
}

/// Splits the provided text into a vector of sentences.
pub fn split_into_sentences(text: &str) -> Vec<String> {
	UnicodeSegmentation::split_sentence_bounds(text)
		.map(|sentence| sentence.trim().to_string())
		.filter(|s| !s.is_empty())
		.collect()
}

/// Splits the provided tokens into chunks based on the maximum token length.
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
				current_chunk = sentence.clone();
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

fn find_entity_indices(sentence: &str, entity: &str) -> Vec<(usize, usize)> {
	let sentence_lower = sentence.to_lowercase();
	let entity_lower = entity.to_lowercase();
	let mut positions = Vec::new();
	let mut start_pos = 0;
	while let Some(pos) = sentence_lower[start_pos..].find(&entity_lower) {
		let start = start_pos + pos;
		let end = start + entity.len();
		positions.push((start, end));
		start_pos = end; // Move past this occurrence
	}

	positions
}

pub async fn tokens_to_words(llm: &dyn LLM, tokens: &[Vec<i32>]) -> Vec<Vec<String>> {
	let mut words_list = Vec::new();
	for token_seq in tokens {
		let words = llm.tokens_to_words(token_seq).await;
		words_list.push(words);
	}
	words_list
}

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
					matched_entities.push((entity.clone(), label.clone(), token_start, token_end));
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

fn find_all_token_indices(tokens: &[String], entity: &str) -> Vec<(usize, usize)> {
	let entity_tokens: Vec<String> = entity.split_whitespace().map(|s| s.to_lowercase()).collect();
	let mut indices = Vec::new();

	for i in 0..tokens.len() {
		let token_slice: Vec<String> = tokens[i..i + entity_tokens.len().min(tokens.len() - i)]
			.iter()
			.map(|s| s.to_lowercase())
			.collect();
		if i + entity_tokens.len() <= tokens.len() && token_slice == entity_tokens[..] {
			indices.push((i, i + entity_tokens.len() - 1));
		}
	}

	indices
}

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

					// Skip pairs where the distance is greater than 15 characters
					if char_distance > 25 {
						continue;
					}

					pairs.push((entity1.clone(), *start1, *end1, entity2.clone(), *start2, *end2));
				}
			}

			results.push(ClassifiedSentenceWithPairs {
				sentence: sentence.clone(),
				entities,
				pairs,
			});
		} else {
			results.push(ClassifiedSentenceWithPairs {
				sentence: sentence.clone(),
				entities,
				pairs: Vec::new(),
			});
		}
	}

	results
}

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

/// Utility function to calculate biased sentence embedding
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

	// Initialize the biased sentence embedding
	let mut biased_sentence_embedding = sentence_embedding.clone();

	// Adjust the sentence embedding with entity embeddings
	for i in 0..biased_sentence_embedding.len() {
		biased_sentence_embedding[i] += head_attention_score * head_embedding[i] +
			tail_attention_score * tail_embedding[i] +
			score * predicate_embedding[i];
	}

	// Normalize the resulting vector (optional)
	let norm: f32 = biased_sentence_embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
	let normalized_biased_sentence_embedding: Vec<f32> =
		biased_sentence_embedding.iter().map(|&x| x / norm).collect();

	Ok(normalized_biased_sentence_embedding)
}

pub fn generate_custom_comb_uuid() -> String {
	let custom_epoch = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
	let now = Utc::now();
	let millis_since_epoch = now.signed_duration_since(custom_epoch).num_milliseconds();

	// Ensure the timestamp fits into 52 bits
	let timestamp_part = (millis_since_epoch as u64) & 0x000F_FFFF_FFFF_FFFF;

	// Generate a 12-bit random number
	let mut rng = thread_rng();
	let random_part: u16 = rng.gen_range(0..4096);

	// Combine both parts: Shift timestamp by 12 bits and add the random part
	let uuid_int = (timestamp_part << 12) | (random_part as u64);

	uuid_int.to_string()
}

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
