use crate::{
	embedded_knowledge, semantic_knowledge, ActualDbPool, StorageError, StorageErrorKind,
	StorageResult,
};
use common::DocumentPayload;
use diesel::{sql_types::BigInt, ExpressionMethods, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use pgvector::{Vector, VectorExpressionMethods};
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
	traverser_results: &[(String, String, String, String, String, String, String, f32)],
	filtered_results: &[(String, String)],
) -> Vec<(String, String)> {
	let mut unique_pairs: HashSet<(String, String)> = filtered_results.iter().cloned().collect();

	for (_id, _doc_id, subject, object, _doc_source, _sentence, _event_id, _score) in
		traverser_results.iter()
	{
		unique_pairs.insert((subject.to_string(), object.to_string()));
	}

	unique_pairs.into_iter().collect()
}

pub fn find_intersection(
	pairs1: &[(String, String)],
	pairs2: &[(String, String)],
) -> Vec<(String, String)> {
	let set1: HashSet<_> = pairs1.iter().cloned().collect();
	let set2: HashSet<_> = pairs2.iter().cloned().collect();
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

pub async fn fetch_documents_for_embedding(
	conn: &mut AsyncPgConnection,
	embedding: &Vec<f32>,
	adjusted_offset: i64,
	limit: i64,
	session_id: &String,
	query: &String,
	collection_id: &String,
	payload: &Vec<f32>,
) -> StorageResult<Vec<DocumentPayload>> {
	let vector = Vector::from(embedding.clone());

	let query_result = embedded_knowledge::dsl::embedded_knowledge
		.select((
			embedded_knowledge::dsl::embeddings,
			embedded_knowledge::dsl::score,
			embedded_knowledge::dsl::event_id,
			embedded_knowledge::dsl::embeddings.cosine_distance(vector.clone()),
		))
		.filter(embedded_knowledge::dsl::embeddings.cosine_distance(vector.clone()).le(0.5))
		.order_by(embedded_knowledge::dsl::embeddings.cosine_distance(vector.clone()))
		.limit(limit)
		.offset(adjusted_offset)
		.load::<(Option<Vector>, f32, String, Option<f64>)>(conn)
		.await;

	match query_result {
		Ok(result) => {
			let mut results = Vec::new();
			for (_embeddings, score, event_id, other_cosine_distance) in result {
				let mut query_semantic = semantic_knowledge::dsl::semantic_knowledge
					.select((
						semantic_knowledge::dsl::document_id,
						semantic_knowledge::dsl::subject,
						semantic_knowledge::dsl::object,
						semantic_knowledge::dsl::document_source,
						semantic_knowledge::dsl::sentence,
						semantic_knowledge::dsl::collection_id,
					))
					.filter(semantic_knowledge::dsl::event_id.eq(event_id))
					.into_boxed();
				if !collection_id.is_empty() {
					query_semantic = query_semantic
						.filter(semantic_knowledge::dsl::collection_id.eq(collection_id));
				}
				let query_result_semantic = query_semantic
					.offset(0)
					.load::<(String, String, String, String, String, Option<String>)>(conn)
					.await;

				match query_result_semantic {
					Ok(result_semantic) => {
						for (
							doc_id,
							subject,
							object,
							document_store,
							sentence,
							collection_id_semantic,
						) in result_semantic
						{
							let mut doc_payload = DocumentPayload::default();
							doc_payload.doc_id = doc_id.clone();
							doc_payload.subject = subject.clone();
							doc_payload.object = object.clone();
							doc_payload.doc_source = document_store.clone();
							doc_payload.cosine_distance = other_cosine_distance;
							doc_payload.score = score;
							doc_payload.sentence = sentence.clone();
							doc_payload.session_id = Some(session_id.clone());
							doc_payload.query_embedding = Some(payload.clone());
							doc_payload.query = Some(query.clone());
							doc_payload.collection_id =
								collection_id_semantic.unwrap_or("".to_string()).to_string();
							results.push(doc_payload);
						}
					},
					Err(e) => {
						error!("Error querying semantic data: {:?}", e);
						return Err(StorageError {
							kind: StorageErrorKind::Query,
							source: Arc::new(anyhow::Error::from(e)),
						});
					},
				}
			}
			Ok(results)
		},
		Err(err) => {
			log::error!("Query failed: {:?}", err);
			Err(StorageError {
				kind: StorageErrorKind::Query,
				source: Arc::new(anyhow::Error::from(err)),
			})
		},
	}
}

use diesel::{
	sql_query,
	sql_types::{Float4, Float8, Text},
	QueryableByName,
};

#[derive(QueryableByName, Debug)]
struct EmbeddingResult {
	#[allow(dead_code)]
	#[diesel(sql_type = Text)]
	embeddings: String,
	#[diesel(sql_type = Float4)]
	score: f32,
	#[diesel(sql_type = Text)]
	event_id: String,
	#[diesel(sql_type = Float8)]
	similarity: f64,
}

#[derive(QueryableByName, Debug)]
struct SemanticResult {
	#[diesel(sql_type = Text)]
	document_id: String,
	#[diesel(sql_type = Text)]
	subject: String,
	#[diesel(sql_type = Text)]
	object: String,
	#[diesel(sql_type = Text)]
	document_source: String,
	#[diesel(sql_type = Text)]
	sentence: String,
	#[diesel(sql_type = Text)]
	collection_id: String,
}

pub async fn fetch_documents_for_embedding_pgembed(
	conn: &mut AsyncPgConnection,
	embedding: &Vec<f32>,
	adjusted_offset: i64,
	limit: i64,
	session_id: &String,
	query: &String,
	collection_id: &String,
	payload: &Vec<f32>,
) -> StorageResult<Vec<DocumentPayload>> {
	let target_vector =
		format!("'{}'", serde_json::to_string(embedding).expect("Failed to format embeddings"));
	println!("target vector ----{:?}", target_vector);
	let query_result = sql_query(&format!(
		r#"
            SELECT array_to_string(embeddings::real[], ',') AS embeddings, 
                   score, 
                   event_id, 
                   (embeddings <=> {})::FLOAT8 AS similarity 
            FROM embedded_knowledge
            WHERE (embeddings <=> {})::FLOAT8 < 0.5
            ORDER BY similarity ASC
            LIMIT $1
            OFFSET $2
        "#,
		target_vector, target_vector
	))
	.bind::<BigInt, _>(limit)
	.bind::<BigInt, _>(adjusted_offset)
	.load::<EmbeddingResult>(conn)
	.await;

	match query_result {
		Ok(results) => {
			let mut payload_results = Vec::new();
			for EmbeddingResult { embeddings: _, score, event_id, similarity } in results {
				println!("This is the similarity ------{:?}", similarity);
				println!("This is the collection_id ------{:?}", collection_id);
				let semantic_query_string = r#"
                    SELECT document_id, subject, object, document_source, sentence, collection_id
                    FROM semantic_knowledge
                    WHERE event_id = $1 
                      AND (COALESCE($2, '') = '' OR collection_id = $2)
                    "#;

				let semantic_query = diesel::sql_query(semantic_query_string)
					.bind::<Text, _>(&event_id)
					.bind::<Text, _>(collection_id);

				let query_result_semantic = semantic_query.load::<SemanticResult>(conn).await;

				match query_result_semantic {
					Ok(result_semantic) => {
						for SemanticResult {
							document_id,
							subject,
							object,
							document_source,
							sentence,
							collection_id,
						} in result_semantic
						{
							let mut doc_payload = DocumentPayload::default();
							doc_payload.doc_id = document_id.clone();
							doc_payload.subject = subject.clone();
							doc_payload.object = object.clone();
							doc_payload.doc_source = document_source.clone();
							doc_payload.cosine_distance = Some(similarity);
							doc_payload.score = score;
							doc_payload.sentence = sentence.clone();
							doc_payload.session_id = Some(session_id.clone());
							doc_payload.query_embedding = Some(payload.clone());
							doc_payload.query = Some(query.clone());
							doc_payload.collection_id = collection_id.clone();
							payload_results.push(doc_payload);
						}
					},
					Err(e) => {
						error!("Error querying semantic data: {:?}", e);
						return Err(StorageError {
							kind: StorageErrorKind::Query,
							source: Arc::new(anyhow::Error::from(e)),
						});
					},
				}
			}
			Ok(payload_results)
		},
		Err(err) => {
			log::error!("Query failed: {:?}", err);
			Err(StorageError {
				kind: StorageErrorKind::Query,
				source: Arc::new(anyhow::Error::from(err)),
			})
		},
	}
}

pub fn parse_vector(embedding_str: Option<String>) -> Option<Vector> {
	embedding_str.map(|s| {
		let float_values: Vec<f32> =
			s.split(',').filter_map(|v| v.trim().parse::<f32>().ok()).collect();
		Vector::from(float_values)
	})
}
