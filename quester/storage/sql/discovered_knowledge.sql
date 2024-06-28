CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE discovered_knowledge(
    id SERIAL PRIMARY KEY,
    doc_id VARCHAR,
    doc_source VARCHAR,
    sentence TEXT,
    subject TEXT,
    object TEXT,
    cosine_distance FLOAT8,
    query_embedding vector(384),
    session_id VARCHAR,
    score FLOAT4
);