CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE embedded_knowledge(
    id SERIAL PRIMARY KEY,
    embeddings vector(384),
    score FLOAT4,
    event_id VARCHAR
);
