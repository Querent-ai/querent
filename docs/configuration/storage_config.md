---
title: Storage configuration
sidebar_position: 1
---

## Storage Configuration

The storage configuration allows you to define and customize the storage providers used by the Querent system. The configuration supports various types of storage, such as index, vector, and graph databases.

### PostgreSQL Configuration

PostgreSQL is used for index storage. You

 can configure the connection details to your PostgreSQL database.

| Property      | Description                           | Example Value                                             |
|---------------|---------------------------------------|-----------------------------------------------------------|
| `name`        | Name of the PostgreSQL configuration  | `querent_test`                                            |
| `storage_type`| Type of storage used                  | `index`                                                   |
| `config.url`  | Connection URL to the PostgreSQL database | `postgres://querent:querent@localhost/querent_test?sslmode=prefer` |

Example:

```yaml
storage_configs:
  postgres:
    name: querent_test
    storage_type: index
    config:
      url: postgres://querent:querent@localhost/querent_test?sslmode=prefer
```

### Milvus Configuration

Milvus is used for vector storage. The configuration allows you to specify the connection details to the Milvus server.

| Property      | Description                           | Example Value                                             |
|---------------|---------------------------------------|-----------------------------------------------------------|
| `name`        | Name of the Milvus configuration      | `semantic_milvus_db`                                      |
| `storage_type`| Type of storage used                  | `vector`                                                  |
| `config.url`  | Connection URL to the Milvus server   | `http://localhost:19530`                                  |
| `config.username` | Username for Milvus server (if required) |                                                           |
| `config.password` | Password for Milvus server (if required) |                                                           |

Example:

```yaml
storage_configs:
  milvus:
    name: semantic_milvus_db
    storage_type: vector
    config:
      url: http://localhost:19530
      username: ""
      password: ""
```

### Neo4j Configuration

Neo4j is used for graph storage. You can configure the connection details to your Neo4j database.

| Property      | Description                           | Example Value                                             |
|---------------|---------------------------------------|-----------------------------------------------------------|
| `name`        | Name of the Neo4j configuration       | `semantic_neo4j_db`                                       |
| `storage_type`| Type of storage used                  | `graph`                                                   |
| `config.db_name` | Name of the Neo4j database          | `neo4j`                                                   |
| `config.url`  | Connection URL to the Neo4j server    | `bolt://localhost:7687`                                   |
| `config.username` | Username for Neo4j server          | `neo4j`                                                   |
| `config.password` | Password for Neo4j server          | `password_neo`                                            |

Example:

```yaml
storage_configs:
  neo4j:
    name: semantic_neo4j_db
    storage_type: graph
    config:
      db_name: neo4j
      url: bolt://localhost:7687
      username: neo4j
      password: password_neo
```

### Example Configuration

Here is an example of a full storage configuration including PostgreSQL, Milvus, and Neo4j:

```yaml
storage_configs:
  postgres:
    name: querent_test
    storage_type: index
    config:
      url: postgres://querent:querent@localhost/querent_test?sslmode=prefer
  milvus:
    name: semantic_milvus_db
    storage_type: vector
    config:
      url: http://localhost:19530
      username: ""
      password: ""
  neo4j:
    name: semantic_neo4j_db
    storage_type: graph
    config:
      db_name: neo4j
      url: bolt://localhost:7687
      username: neo4j
      password: password_neo
```
