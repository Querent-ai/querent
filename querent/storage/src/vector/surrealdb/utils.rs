use std::sync::Arc;

use crate::{StorageError, StorageErrorKind, StorageResult};
use common::DocumentPayload;
use surrealdb::{engine::local::Db, Response, Surreal};

use super::surrealdb::{QueryResultEmbedded, QueryResultSemantic};

pub async fn fetch_documents_for_embedding(
	db: &Surreal<Db>,
	embedding: &Vec<f32>,
	adjusted_offset: i64,
	limit: i64,
	session_id: &String,
	query: &String,
	collection_id: &String,
	payload: &Vec<f32>,
) -> StorageResult<Vec<DocumentPayload>> {
	let query_string = format!(
        "SELECT embeddings, score, event_id, vector::similarity::cosine(embeddings, $embedding) AS cosine_distance 
        FROM embedded_knowledge 
        WHERE vector::similarity::cosine(embeddings, $embedding) > 0.5
        ORDER BY cosine_distance DESC
        LIMIT {} START {}",
        limit, adjusted_offset
    );
	let mut response: Response = db
		.query(query_string)
		.bind(("embedding", embedding))
		.await
		.map_err(|e| StorageError {
			kind: StorageErrorKind::Query,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

	let query_results = response.take::<Vec<QueryResultEmbedded>>(0).map_err(|e| StorageError {
		kind: StorageErrorKind::Internal,
		source: Arc::new(anyhow::Error::from(e)),
	})?;
	let mut results: Vec<DocumentPayload> = Vec::new();
	for query_result in query_results {
		let query_string_semantic = format!(
			"SELECT document_id, subject, object, document_source, sentence, subject_type, object_type
            FROM semantic_knowledge 
            WHERE event_id = '{}' ",
			query_result.event_id.clone()
		);

		let mut response_semantic: Response =
			db.query(query_string_semantic).await.map_err(|e| StorageError {
				kind: StorageErrorKind::Query,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		let query_results_semantic = response_semantic
			.take::<Vec<QueryResultSemantic>>(0)
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		for query_result_semantic in query_results_semantic {
			let mut doc_payload = DocumentPayload::default();
			doc_payload.doc_id = query_result_semantic.document_id.clone();
			doc_payload.subject = query_result_semantic.subject.clone();
			doc_payload.object = query_result_semantic.object.clone();
			doc_payload.doc_source = query_result_semantic.document_source.clone();
			doc_payload.cosine_distance =
				Some(1.0 - query_result.cosine_distance.unwrap_or(0.0).clone());
			doc_payload.score = query_result.score.clone();
			doc_payload.sentence = query_result_semantic.sentence.clone();
			doc_payload.session_id = Some(session_id.clone());
			doc_payload.query_embedding = Some(payload.clone());
			doc_payload.query = Some(query.clone());
			doc_payload.collection_id = collection_id.clone();
			results.push(doc_payload);
		}
	}

	Ok(results)
}
