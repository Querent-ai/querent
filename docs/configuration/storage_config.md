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

Here is an example of a full storage configuration including PostgreSQL, and Neo4j:

```yaml
storage_configs:
  postgres:
    name: querent_test
    storage_type: index
    config:
      url: postgres://querent:querent@localhost/querent_test?sslmode=prefer
  neo4j:
    name: semantic_neo4j_db
    storage_type: graph
    config:
      db_name: neo4j
      url: bolt://localhost:7687
      username: neo4j
      password: password_neo
```
