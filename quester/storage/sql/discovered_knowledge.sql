CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE discovered_knowledge(
    id SERIAL PRIMARY KEY,
    doc_id VARCHAR,
    doc_source VARCHAR,
    sentence TEXT,
    knowledge TEXT,
    subject TEXT,
    object TEXT,
    predicate TEXT,
    cosine_distance FLOAT8,
    query_embedding FLOAT4[],
    session_id TEXT
);