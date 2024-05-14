use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use diesel::{
	result::{ConnectionError, ConnectionResult},
	ExpressionMethods, QueryDsl,
};

use diesel_async::{
	pg::AsyncPgConnection,
	pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager, ManagerConfig},
	scoped_futures::ScopedFutureExt,
	RunQueryDsl,
};
use futures_util::{future::BoxFuture, FutureExt};
use proto::semantics::PostgresConfig;
use std::{sync::Arc, time::SystemTime};
use tracing::error;

use crate::{ActualDbPool, Storage, StorageError, StorageErrorKind, StorageResult, POOL_TIMEOUT};
use pgvector::Vector;

use deadpool::Runtime;
use diesel::{table, Insertable, Queryable, QueryableByName, Selectable};
use diesel_async::AsyncConnection;
use pgvector::VectorExpressionMethods;
use rustls::{
	client::{ServerCertVerified, ServerCertVerifier},
	ServerName,
};

#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = embedded_knowledge)]

pub struct EmbeddedKnowledge {
	pub document_id: String,
	pub document_source: String,
	pub knowledge: String,
	pub embeddings: Option<Vector>,
	pub predicate: String,
	pub sentence: Option<String>,
	pub collection_id: Option<String>,
	pub image_id: Option<String>,
}

#[derive(Queryable, Insertable, Selectable, Debug, Clone, QueryableByName)]
#[diesel(table_name = discovered_knowledge)]
pub struct DiscoveredKnowledge {
	pub doc_id: String,
	pub doc_source: String,
	pub sentence: String,
	pub knowledge: String,
	pub subject: String,
	pub object: String,
	pub predicate: String,
	pub cosine_distance: Option<f64>,
	pub query_embedding: Option<Vector>,
	pub session_id: Option<String>,
}

impl DiscoveredKnowledge {
	pub fn from_document_payload(payload: DocumentPayload) -> Self {
		DiscoveredKnowledge {
			doc_id: payload.doc_id,
			doc_source: payload.doc_source,
			sentence: payload.sentence,
			knowledge: payload.knowledge,
			subject: payload.subject,
			object: payload.object,
			predicate: payload.predicate,
			cosine_distance: payload.cosine_distance,
			query_embedding: Some(Vector::from(payload.query_embedding.unwrap_or_default())),
			session_id: payload.session_id,
		}
	}
}

pub struct PGVector {
	pub pool: ActualDbPool,
	pub config: PostgresConfig,
}

struct NoCertVerifier {}

impl ServerCertVerifier for NoCertVerifier {
	fn verify_server_cert(
		&self,
		_end_entity: &rustls::Certificate,
		_intermediates: &[rustls::Certificate],
		_server_name: &ServerName,
		_scts: &mut dyn Iterator<Item = &[u8]>,
		_ocsp_response: &[u8],
		_now: SystemTime,
	) -> Result<ServerCertVerified, rustls::Error> {
		// Will verify all (even invalid) certs without any checks (sslmode=require)
		Ok(ServerCertVerified::assertion())
	}
}

impl PGVector {
	pub async fn new(config: PostgresConfig) -> StorageResult<Self> {
		let mut d_config = ManagerConfig::default();
		let tls_enabled = config.url.contains("sslmode=require");
		let manager = if tls_enabled {
			// diesel-async does not support any TLS connections out of the box, so we need to manually
			// provide a setup function which handles creating the connection
			d_config.custom_setup = Box::new(establish_connection);
			AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
				config.url.clone(),
				d_config,
			)
		} else {
			AsyncDieselConnectionManager::<AsyncPgConnection>::new(config.url.clone())
		};
		let pool = Pool::builder(manager)
			.max_size(10)
			.wait_timeout(POOL_TIMEOUT)
			.create_timeout(POOL_TIMEOUT)
			.recycle_timeout(POOL_TIMEOUT)
			.runtime(Runtime::Tokio1)
			.build()
			.map_err(|e| StorageError {
				kind: StorageErrorKind::Internal,
				source: Arc::new(anyhow::Error::from(e)),
			})?;

		Ok(PGVector { pool, config })
	}
}

fn establish_connection(config: &str) -> BoxFuture<ConnectionResult<AsyncPgConnection>> {
	let fut = async {
		let rustls_config = rustls::ClientConfig::builder()
			.with_safe_defaults()
			.with_custom_certificate_verifier(Arc::new(NoCertVerifier {}))
			.with_no_client_auth();

		let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
		let (client, conn) = tokio_postgres::connect(config, tls)
			.await
			.map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
		tokio::spawn(async move {
			if let Err(e) = conn.await {
				error!("Database connection failed: {e}");
			}
		});
		AsyncPgConnection::try_from(client).await
	};
	fut.boxed()
}

#[async_trait]
impl Storage for PGVector {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _ = self.pool.get().await?;
		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		conn.transaction::<_, diesel::result::Error, _>(|conn| {
			async move {
				for (document_id, source, image_id, item) in payload {
					let form = EmbeddedKnowledge {
						document_id: document_id.clone(),
						document_source: source.clone(),
						embeddings: Some(Vector::from(item.embeddings.clone())),
						predicate: item.namespace.clone(),
						knowledge: item.id.clone(),
						sentence: item.sentence.clone(),
						collection_id: Some(_collection_id.clone()),
						image_id: image_id.clone(),
					};
					diesel::insert_into(embedded_knowledge::dsl::embedded_knowledge)
						.values(form)
						.execute(conn)
						.await?;
				}
				Ok(())
			}
			.scope_boxed()
		})
		.await
		.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		Ok(())
	}

	async fn insert_discovered_knowledge(
		&self,
		payload: &Vec<DocumentPayload>,
	) -> StorageResult<()> {
		let conn = &mut self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;
		conn.transaction::<_, diesel::result::Error, _>(|conn| {
			async move {
				for item in payload {
					let form = DiscoveredKnowledge::from_document_payload(item.clone());
					diesel::insert_into(discovered_knowledge::dsl::discovered_knowledge)
						.values(form)
						.execute(conn)
						.await?;
				}
				Ok(())
			}
			.scope_boxed()
		})
		.await
		.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		Ok(())
	}

	async fn fetch_discovered_knowledge(
		&self,
		session_id: String,
	) -> StorageResult<Vec<DocumentPayload>> {
		let mut conn = self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let query_result = discovered_knowledge::dsl::discovered_knowledge
			.select((
				discovered_knowledge::dsl::doc_id,
				discovered_knowledge::dsl::doc_source,
				discovered_knowledge::dsl::sentence,
				discovered_knowledge::dsl::knowledge,
				discovered_knowledge::dsl::subject,
				discovered_knowledge::dsl::object,
				discovered_knowledge::dsl::predicate,
				discovered_knowledge::dsl::cosine_distance,
			))
			.filter(discovered_knowledge::dsl::session_id.eq(&session_id))
			.load::<(String, String, String, String, String, String, String, Option<f64>)>(
				&mut *conn,
			)
			.await;

		match query_result {
			Ok(result) => {
				let mut results = Vec::new();
				for (
					doc_id,
					doc_source,
					sentence,
					knowledge,
					subject,
					object,
					predicate,
					cosine_distance,
				) in result
				{
					let doc_payload = DocumentPayload {
						doc_id,
						doc_source,
						sentence,
						knowledge,
						subject,
						object,
						predicate,
						cosine_distance,
						session_id: Some(session_id.clone()),
						query_embedding: None, // Assuming there's no query_embedding in this context
					};

					results.push(doc_payload);
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

	async fn similarity_search_l2(
		&self,
		session_id: String,
		_collection_id: String,
		payload: &Vec<f32>,
		max_results: i32, // Assume this is the total max, e.g., 100
		similarity_threshold: f64,
	) -> StorageResult<Vec<DocumentPayload>> {
		let mut conn = self.pool.get().await.map_err(|e| StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::Error::from(e)),
		})?;

		let vector = Vector::from(payload.clone());
		// First, fetch up to a larger limit, say 100
		let query_result = embedded_knowledge::dsl::embedded_knowledge
			.select((
				embedded_knowledge::dsl::document_id,
				embedded_knowledge::dsl::document_source,
				embedded_knowledge::dsl::knowledge,
				embedded_knowledge::dsl::embeddings,
				embedded_knowledge::dsl::predicate,
				embedded_knowledge::dsl::sentence,
				embedded_knowledge::dsl::embeddings.cosine_distance(vector.clone()),
			))
			.filter(
				embedded_knowledge::dsl::embeddings
					.cosine_distance(vector.clone())
					.le(similarity_threshold),
			)
			.order_by(embedded_knowledge::dsl::embeddings.cosine_distance(vector))
			.limit(max_results as i64)
			.load::<(String, String, String, Option<Vector>, String, Option<String>, Option<f64>)>(
				&mut *conn,
			)
			.await;

		match query_result {
			Ok(result) => {
				let mut results = Vec::new();
				let mut unique_sentences = std::collections::HashSet::new();
				for (
					doc_id,
					doc_source,
					knowledge,
					_embeddings,
					predicate,
					sentence,
					cosine_distance,
				) in result
				{
					if let Some(ref sent) = sentence {
						if !unique_sentences.insert(sent.clone()) {
							continue;
						}
					}

					let mut doc_payload = DocumentPayload::default();
					doc_payload.doc_id = doc_id;
					doc_payload.knowledge = knowledge;
					let knowledge_parts: Vec<&str> = doc_payload.knowledge.split('-').collect();
					if knowledge_parts.len() == 3 {
						doc_payload.subject = knowledge_parts[0].to_string();
						doc_payload.predicate = knowledge_parts[1].to_string();
						doc_payload.object = knowledge_parts[2].to_string();
					}
					doc_payload.doc_source = doc_source;
					doc_payload.cosine_distance = cosine_distance;
					doc_payload.predicate = predicate;
					if let Some(sentence_value) = sentence {
						doc_payload.sentence = sentence_value;
					}
					doc_payload.session_id = Some(session_id.clone());
					doc_payload.query_embedding = Some(payload.clone());
					results.push(doc_payload);

					if results.len() == 10 {
						break;
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

	async fn insert_graph(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	async fn index_knowledge(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		Ok(())
	}

	/// Store key value pair
	async fn store_kv(&self, _key: &String, _value: &String) -> StorageResult<()> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}

	/// Get value for key
	async fn get_kv(&self, _key: &String) -> StorageResult<Option<String>> {
		Err(StorageError {
			kind: StorageErrorKind::Internal,
			source: Arc::new(anyhow::anyhow!("Not implemented")),
		})
	}
}

table! {
	use diesel::sql_types::*;
	use pgvector::sql_types::*;

	embedded_knowledge (id) {
		id -> Int4,
		document_id -> Varchar,
		document_source -> Varchar,
		knowledge -> Text,
		embeddings -> Nullable<Vector>,
		predicate -> Text,
		sentence -> Nullable<Text>,
		collection_id -> Nullable<Text>,
		image_id -> Nullable<Text>,
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
		knowledge -> Text,
		subject -> Text,
		object -> Text,
		predicate -> Text,
		cosine_distance -> Nullable<Float8>,
		query -> Nullable<Text>,
		query_embedding -> Nullable<Vector>,
		session_id -> Nullable<Text>,
	}
}

#[cfg(test)]
mod test {
	use super::*;
	// use diesel::sql_types::*;
	// use diesel::QueryableByName;
	use proto::semantics::StorageType;

	// #[derive(Debug, QueryableByName)]
	// pub struct ExistsResult {
	// 	#[sql_type = "Bool"]
	// 	pub exists: bool,
	// }
	const TEST_DB_URL: &str = "postgres://querent:querent@localhost/querent_test?sslmode=prefer";

	// async fn setup_test_environment() -> PGVector {
	// 	// Create a PGVector instance with the test database URL
	// 	let config = PostgresConfig {
	// 		url: TEST_DB_URL.to_string(),
	// 		name: "test".to_string(),
	// 		storage_type: Some(StorageType::Vector),
	// 	};

	// 	PGVector::new(config).await.expect("Failed to create PGVector instance")
	// }

	// Test function
	// #[tokio::test]

	// async fn test_similarity_search_l2() {
	// 	let storage = setup_test_environment().await;
	// 	let test_vector = vec![
	// 		-0.05037749,
	// 		-0.0009511291,
	// 		0.12806408,
	// 		0.018326044,
	// 		0.06526254,
	// 		-0.027956909,
	// 		0.003987734,
	// 		0.048739523,
	// 		-0.060903423,
	// 		-0.074991,
	// 		-0.09573072,
	// 		0.050821844,
	// 		-0.021284636,
	// 		-0.04397921,
	// 		-0.05777003,
	// 		-0.05155943,
	// 		0.087651886,
	// 		0.0074979197,
	// 		-0.0030403086,
	// 		0.016083442,
	// 		0.10646215,
	// 		0.022409316,
	// 		0.022400374,
	// 		-0.040489018,
	// 		-0.033232633,
	// 		0.09977804,
	// 		-0.032748625,
	// 		0.035224825,
	// 		0.07842346,
	// 		-0.032558076,
	// 		-0.010961892,
	// 		-0.023246635,
	// 		-0.07460568,
	// 		-0.056840613,
	// 		0.025877716,
	// 		0.06824206,
	// 		-0.08720005,
	// 		0.022638632,
	// 		0.00938228,
	// 		0.07535303,
	// 		-0.04815581,
	// 		-0.1264695,
	// 		-0.009800368,
	// 		-0.06974712,
	// 		0.06570513,
	// 		-0.06473228,
	// 		0.02294531,
	// 		0.032823928,
	// 		-0.040719036,
	// 		-0.052740302,
	// 		0.080524586,
	// 		-0.044041883,
	// 		-0.015854549,
	// 		-0.046358135,
	// 		-0.07267197,
	// 		-0.06012131,
	// 		-0.09751008,
	// 		-0.022398466,
	// 		0.005739892,
	// 		-0.06786603,
	// 		-0.004023296,
	// 		0.084576294,
	// 		0.038820248,
	// 		-0.001814028,
	// 		0.04659332,
	// 		0.014355912,
	// 		0.060716733,
	// 		-0.09807369,
	// 		-0.010609437,
	// 		0.0075140116,
	// 		0.0121418685,
	// 		0.113383815,
	// 		-0.08141415,
	// 		-0.029830258,
	// 		0.019126514,
	// 		0.06231785,
	// 		-0.10179203,
	// 		0.06053511,
	// 		-0.02577678,
	// 		-0.024837011,
	// 		0.032675825,
	// 		-0.092230685,
	// 		0.04196277,
	// 		-0.077769585,
	// 		-0.038244426,
	// 		-0.004516286,
	// 		0.024928773,
	// 		-0.057323217,
	// 		-0.014408102,
	// 		-0.04306887,
	// 		-0.07073347,
	// 		0.08972742,
	// 		-0.097102724,
	// 		0.06262324,
	// 		-0.0137449615,
	// 		-0.016806819,
	// 		0.07591239,
	// 		0.0142037645,
	// 		-0.004048466,
	// 		-0.007972133,
	// 		-0.025862144,
	// 		0.0113745835,
	// 		-0.11197646,
	// 		-0.086311005,
	// 		-0.014750308,
	// 		-0.02382184,
	// 		0.023554629,
	// 		-0.050376866,
	// 		-0.05962369,
	// 		-0.009094023,
	// 		0.006791366,
	// 		-0.037500493,
	// 		0.056325905,
	// 		0.014443507,
	// 		-0.07159452,
	// 		-0.046980057,
	// 		-0.10951292,
	// 		0.01075155,
	// 		-0.0366402,
	// 		-0.013695508,
	// 		-0.041622087,
	// 		0.038004007,
	// 		0.03246485,
	// 		-0.016438799,
	// 		0.0031957887,
	// 		-0.024614528,
	// 		0.070864506,
	// 		4.5430276e-34,
	// 		-0.019068414,
	// 		-0.092727736,
	// 		-0.09719519,
	// 		0.0020355517,
	// 		-0.025799362,
	// 		0.0051276824,
	// 		-0.017465834,
	// 		-0.036829572,
	// 		-0.08530595,
	// 		-0.017748903,
	// 		-0.039137308,
	// 		0.08954675,
	// 		-0.039896145,
	// 		0.027310634,
	// 		-0.041889608,
	// 		0.025641305,
	// 		0.04277774,
	// 		0.025104547,
	// 		-0.03928549,
	// 		-0.03812894,
	// 		-0.041154858,
	// 		-0.019045737,
	// 		0.0076602544,
	// 		0.026643785,
	// 		0.0056067063,
	// 		0.03653668,
	// 		-0.013934698,
	// 		-0.014346748,
	// 		0.015887639,
	// 		0.023453355,
	// 		0.008143482,
	// 		0.012904039,
	// 		0.029282281,
	// 		-0.0037896892,
	// 		-0.027132077,
	// 		0.07897843,
	// 		0.026628502,
	// 		-0.0130432295,
	// 		-0.026644928,
	// 		-0.055764817,
	// 		-0.024191327,
	// 		-0.03704887,
	// 		0.078350246,
	// 		-0.0600266,
	// 		-0.05512794,
	// 		0.01975641,
	// 		0.03551779,
	// 		-0.018095702,
	// 		-0.03787124,
	// 		-0.062212337,
	// 		-0.0073550064,
	// 		0.05957357,
	// 		-0.01064011,
	// 		0.014705109,
	// 		-0.015239633,
	// 		-0.015908487,
	// 		0.019885933,
	// 		-0.063733175,
	// 		-0.059300676,
	// 		0.059777625,
	// 		-0.039832287,
	// 		0.10769121,
	// 		-0.0012626846,
	// 		-0.0049915966,
	// 		0.0071706143,
	// 		0.037874307,
	// 		-0.0018498041,
	// 		0.0812364,
	// 		0.03582743,
	// 		0.023544503,
	// 		-0.0020088875,
	// 		-0.054698456,
	// 		0.04411253,
	// 		0.052167185,
	// 		-0.04468567,
	// 		-0.029307012,
	// 		0.07849996,
	// 		-0.020717707,
	// 		0.017023839,
	// 		0.051869012,
	// 		-0.04232036,
	// 		-0.028955676,
	// 		-0.0021484955,
	// 		-0.05676299,
	// 		-0.12078888,
	// 		0.054997146,
	// 		0.014592436,
	// 		-0.041710623,
	// 		0.06595448,
	// 		0.02617423,
	// 		-0.012899758,
	// 		-0.0322707,
	// 		0.00858374,
	// 		-0.046195135,
	// 		0.08184964,
	// 		-3.265688e-33,
	// 		0.08949582,
	// 		0.019554265,
	// 		-0.05341951,
	// 		0.063056976,
	// 		0.08856248,
	// 		0.11449219,
	// 		0.09966456,
	// 		0.04483336,
	// 		-0.004012464,
	// 		-0.04879996,
	// 		-0.028572533,
	// 		0.019543706,
	// 		-0.007602821,
	// 		-0.063310824,
	// 		-0.053214718,
	// 		-0.0023427317,
	// 		-0.07522848,
	// 		-0.10013586,
	// 		-0.00023712793,
	// 		-0.0019073216,
	// 		0.0036581506,
	// 		-0.0016208128,
	// 		0.059218075,
	// 		0.08405473,
	// 		-0.00312277,
	// 		-0.04490767,
	// 		-0.04975002,
	// 		-0.063133314,
	// 		-0.10903357,
	// 		-0.0064773173,
	// 		-0.012417073,
	// 		0.052679706,
	// 		-0.06568253,
	// 		0.0008430707,
	// 		-0.016517464,
	// 		0.04499238,
	// 		0.019447936,
	// 		0.03432297,
	// 		-0.021249427,
	// 		-0.01697445,
	// 		0.023137901,
	// 		-0.045170125,
	// 		0.01545105,
	// 		0.03585911,
	// 		0.043401554,
	// 		0.0499458,
	// 		0.064607985,
	// 		-0.09575299,
	// 		0.027072964,
	// 		0.011570742,
	// 		0.039269853,
	// 		0.010695546,
	// 		0.08939195,
	// 		0.07595094,
	// 		0.049765863,
	// 		0.03973702,
	// 		-0.0035750708,
	// 		-0.03775314,
	// 		-0.07525945,
	// 		0.06956788,
	// 		0.026735011,
	// 		0.046054006,
	// 		0.001799498,
	// 		0.06047589,
	// 		0.058661938,
	// 		-0.07017732,
	// 		-0.064067416,
	// 		0.0011935044,
	// 		0.052523103,
	// 		0.017021038,
	// 		0.04253839,
	// 		-0.02212089,
	// 		0.07083487,
	// 		0.057787854,
	// 		0.05734309,
	// 		0.03478251,
	// 		-0.011611904,
	// 		0.01665036,
	// 		-0.107629105,
	// 		-0.01851057,
	// 		0.01726637,
	// 		0.092516206,
	// 		-0.008972013,
	// 		0.0023486742,
	// 		0.09547054,
	// 		-0.02935107,
	// 		-0.017049113,
	// 		-0.07938774,
	// 		0.06534824,
	// 		0.004007366,
	// 		0.028388193,
	// 		-0.022250228,
	// 		-0.08418453,
	// 		-0.006879231,
	// 		-0.029654462,
	// 		-3.81913e-08,
	// 		-0.040030677,
	// 		0.036210153,
	// 		0.033101335,
	// 		0.056511723,
	// 		-0.03795287,
	// 		-0.00472106,
	// 		0.09813975,
	// 		0.07461815,
	// 		0.021844547,
	// 		-0.13070269,
	// 		-0.068085015,
	// 		-0.025587084,
	// 		0.061135743,
	// 		0.012578894,
	// 		0.03749595,
	// 		0.010589201,
	// 		-0.04311323,
	// 		0.051616907,
	// 		-0.005523684,
	// 		-0.03480247,
	// 		0.03995189,
	// 		-0.05161598,
	// 		0.031986915,
	// 		0.024792528,
	// 		0.04402935,
	// 		0.023478277,
	// 		0.058083195,
	// 		-0.031156847,
	// 		-0.0491271,
	// 		0.05990224,
	// 		0.01985537,
	// 		-0.05004319,
	// 		0.038217403,
	// 		0.05139155,
	// 		0.11262835,
	// 		0.058185052,
	// 		0.008504766,
	// 		0.041406818,
	// 		-0.08306079,
	// 		0.019062754,
	// 		-0.040315557,
	// 		0.07881687,
	// 		0.046869166,
	// 		-0.055481438,
	// 		-0.016521903,
	// 		0.053089414,
	// 		-0.10823624,
	// 		0.051719572,
	// 		0.012246474,
	// 		0.016081138,
	// 		-0.045637656,
	// 		0.03136851,
	// 		0.017018983,
	// 		0.034483705,
	// 		0.05789477,
	// 		-0.07391275,
	// 		0.005769378,
	// 		0.07708997,
	// 		-0.026763353,
	// 		-0.011719101,
	// 		-0.00037045093,
	// 		0.03379138,
	// 		0.06889972,
	// 		-0.0029939266,
	// 	];
	// 	let max_results = 5;
	// 	let results = storage
	// 		.similarity_search_l2("collection_id".to_string(), &test_vector, max_results)
	// 		.await;
	// 	println!("Results: {:?}", results);
	// 	assert!(results.is_ok());

	// 	let documents = results.unwrap();
	// 	assert!(!documents.is_empty(), "The search should return at least one result.");
	// }

	// #[tokio::test]
	// async fn test_create_embeddings_index() {
	//     let pg_vector = setup_test_environment().await;

	//     // Attempt to create the index
	//     let result = pg_vector.create_embeddings_index().await;
	//     assert!(result.is_ok());

	// }

	#[tokio::test]
	async fn test_postgres_storage() {
		// Create a postgres config
		let config = PostgresConfig {
			url: TEST_DB_URL.to_string(),
			name: "test".to_string(),
			storage_type: Some(StorageType::Index),
		};

		// Create a PostgresStorage instance with the test database URL
		let storage_result = PGVector::new(config).await;

		// Ensure that the storage is created successfully
		assert!(storage_result.is_ok());

		// Get the storage instance from the result
		let storage = storage_result.unwrap();

		// Perform a connectivity check
		let _connectivity_result = storage.check_connectivity().await;
		// Ensure that the connectivity check is successful
		// Works when there is a database running on the test database URL
		//assert!(_connectivity_result.is_ok());

		// You can add more test cases or assertions based on your specific requirements
	}
}
