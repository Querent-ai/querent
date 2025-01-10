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

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use std::sync::Arc;

use crate::{StorageError, StorageErrorKind, StorageResult};
use common::DocumentPayload;
use surrealdb::{engine::local::Db, Response, Surreal};

use super::surrealdb::{QueryResultEmbedded, QueryResultSemantic};

pub async fn fetch_documents_for_embedding(
	db: &Surreal<Db>,
	embedding: Vec<f32>,
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
