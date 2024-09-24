use common::DocumentPayload;
use diesel::{
	sql_types::{Double, Nullable, Text},
	table, Insertable, Queryable, QueryableByName, Selectable,
};
use pgvector::Vector;
#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = embedded_knowledge)]
pub struct EmbeddedKnowledge {
	pub embeddings: Option<Vector>,
	pub score: f32,
	pub event_id: String,
}

#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = insight_knowledge)]
pub struct InsightKnowledge {
	pub query: Option<String>,
	pub session_id: Option<String>,
	pub response: Option<String>,
}

#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = discovered_knowledge)]
pub struct DiscoveredKnowledge {
	pub doc_id: String,
	pub doc_source: String,
	pub sentence: String,
	pub subject: String,
	pub object: String,
	pub cosine_distance: Option<f64>,
	pub query_embedding: Option<Vector>,
	pub query: Option<String>,
	pub session_id: Option<String>,
	pub score: Option<f64>,
	pub collection_id: String,
}

impl DiscoveredKnowledge {
	pub fn from_document_payload(payload: DocumentPayload) -> Self {
		DiscoveredKnowledge {
			doc_id: payload.doc_id,
			doc_source: payload.doc_source,
			sentence: payload.sentence,
			subject: payload.subject,
			object: payload.object,
			cosine_distance: payload.cosine_distance,
			query_embedding: Some(Vector::from(payload.query_embedding.unwrap_or_default())),
			query: payload.query,
			session_id: payload.session_id,
			score: Some(payload.score as f64),
			collection_id: payload.collection_id,
		}
	}
}

#[derive(QueryableByName)]
pub struct FilteredResults {
	#[diesel(sql_type = diesel::sql_types::Text)]
	pub document_id: String,
	#[diesel(sql_type = diesel::sql_types::Text)]
	pub subject: String,
	#[diesel(sql_type = diesel::sql_types::Text)]
	pub object: String,
	#[diesel(sql_type = diesel::sql_types::Text)]
	pub document_source: String,
	#[diesel(sql_type = diesel::sql_types::Text)]
	pub sentence: String,
	#[diesel(sql_type = diesel::sql_types::Float)]
	pub score: f32,
	#[diesel(sql_type = pgvector::sql_types::Vector)]
	pub embeddings: Vector,
}

table! {
	use diesel::sql_types::*;
	use pgvector::sql_types::*;

	embedded_knowledge (id) {
		id -> Int4,
		embeddings -> Nullable<Vector>,
		score -> Float4,
		event_id -> VarChar,
	}
}

table! {
	use diesel::sql_types::*;
	use pgvector::sql_types::*;

	insight_knowledge (id) {
		id -> Int4,
		query -> Nullable<Text>,
		session_id -> Nullable<Text>,
		response -> Nullable<Text>,

	}
}

table! {
	use diesel::sql_types::*;
	use pgvector::sql_types::*;

	discovered_knowledge (id) {
		id -> Int4,
		doc_id -> Varchar,
		doc_source -> Varchar,
		sentence -> Text,
		subject -> Text,
		object -> Text,
		cosine_distance -> Nullable<Float8>,
		query_embedding -> Nullable<Vector>,
		query -> Nullable<Text>,
		session_id -> Nullable<Text>,
		score -> Nullable<Float8>,
		collection_id -> Text,
	}
}

#[derive(QueryableByName)]

pub struct DiscoveredKnowledgeRaw {
	#[diesel(sql_type = Text)]
	pub doc_id: String,
	#[diesel(sql_type = Text)]
	pub doc_source: String,
	#[diesel(sql_type = Text)]
	pub sentence: String,
	#[diesel(sql_type = Text)]
	pub subject: String,
	#[diesel(sql_type = Text)]
	pub object: String,
	#[diesel(sql_type = Nullable<Double>)]
	pub cosine_distance: Option<f64>,
	#[diesel(sql_type = Nullable<Text>)]
	pub session_id: Option<String>,
	#[diesel(sql_type = Nullable<Double>)]
	pub score: Option<f64>,
	#[diesel(sql_type = Nullable<Text>)]
	pub query: Option<String>,
	#[diesel(sql_type = Nullable<Text>)]
	pub query_embedding: Option<String>,
	#[diesel(sql_type = Text)]
	pub collection_id: String,
}
