DEFINE TABLE  semantic_knowledge SCHEMAFULL;

DEFINE FIELD id ON TABLE semantic_knowledge TYPE any;
DEFINE FIELD subject ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD subject_type ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD object ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD object_type ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD sentence ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD document_id ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD document_source ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD collection_id ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD image_id ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD event_id ON TABLE semantic_knowledge TYPE string;
DEFINE FIELD source_id ON TABLE semantic_knowledge TYPE string;





DEFINE TABLE  embedded_knowledge SCHEMAFULL;

DEFINE FIELD id ON TABLE embedded_knowledge TYPE any;
DEFINE FIELD embeddings ON TABLE embedded_knowledge TYPE array<float>;
DEFINE FIELD score ON TABLE embedded_knowledge TYPE float;
DEFINE FIELD event_id ON TABLE embedded_knowledge TYPE string;




DEFINE TABLE  discovered_knowledge SCHEMAFULL;

DEFINE FIELD id ON TABLE discovered_knowledge TYPE any;
DEFINE FIELD doc_id ON TABLE discovered_knowledge TYPE string;
DEFINE FIELD doc_source ON TABLE discovered_knowledge TYPE string;
DEFINE FIELD sentence ON TABLE discovered_knowledge TYPE string;
DEFINE FIELD subject ON TABLE discovered_knowledge TYPE string;
DEFINE FIELD object ON TABLE discovered_knowledge TYPE string;
DEFINE FIELD cosine_distance ON TABLE discovered_knowledge TYPE float;
DEFINE FIELD query_embedding ON TABLE discovered_knowledge TYPE array<float>;
DEFINE FIELD query ON TABLE discovered_knowledge TYPE string;
DEFINE FIELD session_id ON TABLE discovered_knowledge TYPE string;
DEFINE FIELD score ON TABLE discovered_knowledge TYPE float;