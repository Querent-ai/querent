use crate::{
	embedded_knowledge, semantic_knowledge, ActualDbPool, StorageError, StorageErrorKind,
	StorageResult,
};
use common::DocumentPayload;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};
use tracing::error;

/// Function to get top k entries based on cosine distance and return unique pairs
pub fn get_top_k_pairs(payloads: Vec<DocumentPayload>, k: usize) -> Vec<(String, String)> {
	let mut unique_entries = HashSet::new();
	let mut unique_payloads: Vec<_> = payloads
		.into_iter()
		.filter(|p| {
			unique_entries.insert((p.subject.clone(), p.object.clone())) &&
				p.cosine_distance <= Some(0.2)
		})
		.collect();
	unique_payloads.sort_by(|a, b| {
		a.cosine_distance
			.partial_cmp(&b.cosine_distance)
			.unwrap_or(std::cmp::Ordering::Equal)
	});

	unique_payloads.into_iter().take(k).map(|p| (p.subject, p.object)).collect()
}

pub async fn traverse_node(
	pool: &ActualDbPool,
	node: String,
	combined_results: &mut Vec<(String, String, String, String, String, String, String, f32)>,
	visited_pairs: &mut HashSet<(String, String)>,
	conn: &mut AsyncPgConnection,
	depth: usize,
	direction: &str,
) -> StorageResult<()> {
	if depth >= 1 {
		return Ok(());
	}
	if direction == "inward" {
		let inward_query_result = semantic_knowledge::dsl::semantic_knowledge
			.select((
				semantic_knowledge::dsl::id,
				semantic_knowledge::dsl::document_id,
				semantic_knowledge::dsl::subject,
				semantic_knowledge::dsl::object,
				semantic_knowledge::dsl::document_source,
				semantic_knowledge::dsl::sentence,
				semantic_knowledge::dsl::event_id,
			))
			.filter(semantic_knowledge::dsl::object.eq(&node))
			.load::<(i32, String, String, String, String, String, String)>(conn)
			.await;
		match inward_query_result {
			Ok(results) =>
				for result in results {
					let (id, doc_id, subject, object, doc_source, sentence, event_id) = result;
					let score_query_result = embedded_knowledge::dsl::embedded_knowledge
						.select(embedded_knowledge::dsl::score)
						.filter(embedded_knowledge::dsl::event_id.eq(&event_id))
						.first::<f32>(conn)
						.await;

					match score_query_result {
						Ok(score) =>
							if visited_pairs.insert((subject.clone(), object.clone())) {
								combined_results.push((
									id.to_string(),
									doc_id,
									subject.clone(),
									object.clone(),
									doc_source,
									sentence,
									event_id,
									score,
								));
								let mut new_conn = pool.get().await.map_err(|e| StorageError {
									kind: StorageErrorKind::Internal,
									source: Arc::new(anyhow::Error::from(e)),
								})?;
								Box::pin(traverse_node(
									pool,
									subject,
									combined_results,
									visited_pairs,
									&mut new_conn,
									depth + 1,
									direction,
								))
								.await?;
							},
						Err(e) => {
							error!("Error querying score for event_id {}: {:?}", event_id, e);
						},
					}
				},
			Err(e) => {
				error!("Error querying inward edges for node {}: {:?}", node, e);
				return Err(StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(e)),
				});
			},
		}
	}
	if direction == "outward" {
		let outward_query_result = semantic_knowledge::dsl::semantic_knowledge
			.select((
				semantic_knowledge::dsl::id,
				semantic_knowledge::dsl::document_id,
				semantic_knowledge::dsl::subject,
				semantic_knowledge::dsl::object,
				semantic_knowledge::dsl::document_source,
				semantic_knowledge::dsl::sentence,
				semantic_knowledge::dsl::event_id,
			))
			.filter(semantic_knowledge::dsl::subject.eq(&node))
			.load::<(i32, String, String, String, String, String, String)>(conn)
			.await;

		match outward_query_result {
			Ok(results) =>
				for result in results {
					let (id, doc_id, subject, object, doc_source, sentence, event_id) = result;
					let score_query_result = embedded_knowledge::dsl::embedded_knowledge
						.select(embedded_knowledge::dsl::score)
						.filter(embedded_knowledge::dsl::event_id.eq(&event_id))
						.first::<f32>(conn)
						.await;

					match score_query_result {
						Ok(score) =>
							if visited_pairs.insert((subject.clone(), object.clone())) {
								combined_results.push((
									id.to_string(),
									doc_id,
									subject.clone(),
									object.clone(),
									doc_source,
									sentence,
									event_id,
									score,
								));
								let mut new_conn = pool.get().await.map_err(|e| StorageError {
									kind: StorageErrorKind::Internal,
									source: Arc::new(anyhow::Error::from(e)),
								})?;
								Box::pin(traverse_node(
									pool,
									object,
									combined_results,
									visited_pairs,
									&mut new_conn,
									depth + 1,
									direction,
								))
								.await?;
							},
						Err(e) => {
							error!("Error querying score for event_id {}: {:?}", event_id, e);
						},
					}
				},
			Err(e) => {
				error!("Error querying outward edges for node {}: {:?}", node, e);
				return Err(StorageError {
					kind: StorageErrorKind::Query,
					source: Arc::new(anyhow::Error::from(e)),
				});
			},
		}
	}

	Ok(())
}

pub fn extract_unique_pairs(
	traverser_results: Vec<(String, String, String, String, String, String, String, f32)>,
	filtered_results: Vec<(String, String)>,
) -> Vec<(String, String)> {
	let mut unique_pairs: HashSet<(String, String)> = filtered_results.into_iter().collect();

	for (_id, _doc_id, subject, object, _doc_source, _sentence, _event_id, _score) in
		traverser_results
	{
		unique_pairs.insert((subject.clone(), object.clone()));
	}

	unique_pairs.into_iter().collect()
}

pub fn find_intersection(
	pairs1: Vec<(String, String)>,
	pairs2: Vec<(String, String)>,
) -> Vec<(String, String)> {
	let set1: HashSet<(String, String)> = pairs1.into_iter().collect();
	let set2: HashSet<(String, String)> = pairs2.into_iter().collect();
	set1.intersection(&set2).cloned().collect()
}

pub fn process_traverser_results(
	data: Vec<(i32, String, String, String, String, String, String, f32)>,
) -> Vec<String> {
	// Create a map to store the subject-object pairs and their corresponding results
	let mut pairs_map: HashMap<
		(String, String),
		Vec<(i32, String, String, String, String, String, String, f32)>,
	> = HashMap::new();

	// Populate the map with results
	for (id, doc_id, subject, object, doc_source, sentence, event_id, score) in data {
		pairs_map
			.entry((subject.clone(), object.clone()))
			.or_insert_with(Vec::new)
			.push((id, doc_id, subject, object, doc_source, sentence, event_id, score));
	}

	// Create a vector to store the formatted output
	let mut formatted_output: Vec<String> = Vec::new();

	// Process each pair
	for ((subject, object), mut results) in pairs_map {
		// Sort the results by score in descending order
		results.sort_by(|a, b| b.7.partial_cmp(&a.7).unwrap());

		// Ensure unique subject, object, and sentence combinations and collect top 3
		let mut unique_entries = HashSet::new();
		let mut top_3 = Vec::new();

		for result in results {
			if unique_entries.insert((result.2.clone(), result.3.clone(), result.5.clone())) {
				// Check if the combination is unique
				top_3.push(result);
				if top_3.len() == 3 {
					break;
				}
			}
		}

		// Format the output for each of the top 3 results
		for (i, (id, _doc_id, _subject, _object, _doc_source, sentence, _event_id, _score)) in
			top_3.iter().enumerate()
		{
			formatted_output.push(format!(
				"{}. {} -> {}: {{ sentence: \"{}\", id: \"{}\" }}",
				i + 1,
				subject,
				object,
				sentence,
				id
			));
		}
	}

	formatted_output
}
