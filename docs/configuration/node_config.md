---
title: Node configuration
sidebar_position: 1
---

The node configuration allows you to customize and optimize the settings for individual nodes in your cluster. It is divided into several sections:

- Common configuration settings: shared top-level properties
- Storage settings: defined in the [storage](#storage-configuration) section

A commented example is available here: [querent.config.yaml](https://github.com/querent-ai/querent/blob/main/config/querent.config.yaml).

## Common configuration

| Property           | Description                                                                                       | Env variable              | Default value                |
|--------------------|---------------------------------------------------------------------------------------------------|---------------------------|------------------------------|
| `version`          | Config file version. `0.1`. Or any versioning needed for node                                     |                           |                              |
| `cluster_id`       | Unique identifier of the cluster the node will be joining. Clusters sharing the same network should use distinct cluster IDs.| `QUERENT_CLUSTER_ID`      | `querent-default-cluster`    |
| `node_id`          | Unique identifier of the node. It must be distinct from the node IDs of its cluster peers. Defaults to the instance's short hostname if not set. | `QUERENT_NODE_ID`         | short hostname               |
| `listen_address`   | The IP address or hostname that Querent service binds to for starting REST and GRPC server and connecting this node to other nodes. By default, Querent binds itself to 127.0.0.1 (localhost). This default is not valid when trying to form a cluster. | `QUERENT_LISTEN_ADDRESS`  | `127.0.0.1`                  |
| `advertise_address`| IP address advertised by the node, i.e., the IP address that peer nodes should use to connect to the node for RPCs. | `QUERENT_ADVERTISE_ADDRESS` | `listen_address`           |
| `gossip_listen_port`| The port which to listen for the Gossip cluster membership service (UDP).                         | `QUERENT_GOSSIP_LISTEN_PORT`| `rest.listen_port`          |
| `grpc_listen_port` | The port on which gRPC services listen for traffic.                                               | `QW_GRPC_LISTEN_PORT`     | `rest.listen_port + 1`       |
| `peer_seeds`       | List of IP addresses or hostnames used to bootstrap the cluster and discover the complete set of nodes. This list may contain the current node address and does not need to be exhaustive. | `QUERENT_PEER_SEEDS`     |                              |
| `data_dir`         | Path to directory where data (tmp data, splits kept for caching purpose) is persisted. This is mostly used in indexing. | `QUERENT_DATA_DIR`        | `./querent_data`             |
|                    | Log level of Querent. Can be a direct log level, or a comma separated list of `module_name=level`  | `RUST_LOG`                | `info`                       |

## REST configuration

This section contains the REST API configuration options.

| Property            | Description                                                                                      | Env variable               | Default value |
|---------------------|--------------------------------------------------------------------------------------------------|----------------------------|---------------|
| `listen_port`       | The port on which the REST API listens for HTTP traffic.                                         | `QUERENT_REST_LISTEN_PORT` | `1111`        |
| `cors_allow_origins`| Configure the CORS origins which are allowed to access the API. [Read more](#configuring-cors-cross-origin-resource-sharing) |                            |               |
| `extra_headers`     | List of header names and values                                                                  |                            |               |

### Configuring CORS (Cross-origin resource sharing)

CORS (Cross-origin resource sharing) describes which address or origins can access the REST API from the browser.
By default, sharing resources cross-origin is not allowed.

A wildcard, single origin, or multiple origins can be specified as part of the `cors_allow_origins` parameter:

Example of a REST configuration:

```yaml
rest:
  listen_port: 1789
  extra_headers:
    x-header-1: header-value-1
    x-header-2: header-value-2
  cors_allow_origins: '*'
  # cors_allow_origins: https://my-hdfs-logs.domain.com   # Optionally we can specify one domain
  # cors_allow_origins:                                   # Or allow multiple origins
  #   - https://my-hdfs-logs.domain.com
  #   - https://my-hdfs.other-domain.com
```

### Storage Configuration

The storage configuration allows you to define and customize the storage providers used by the Querent system. The configuration supports various types of storage, such as index, vector, and graph databases.

### PostgreSQL Configuration

PostgreSQL is used for index storage. You can configure the connection details to your PostgreSQL database.

| Property     | Description                | Example Value                                             |
|--------------|----------------------------|-----------------------------------------------------------|
| `name`       | Name of the PostgreSQL configuration   | `querent_test`                                            |
| `storage_type`| Type of storage used       | `index`                                                   |
| `config.url` | Connection URL to the PostgreSQL database | `postgres://querent:querent@localhost/querent_test?sslmode=prefer` |

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

| Property     | Description                | Example Value                                             |
|--------------|----------------------------|-----------------------------------------------------------|
| `name`       | Name of the Milvus configuration    | `semantic_milvus_db`                                      |
| `storage_type`| Type of storage used       | `vector`                                                  |
| `config.url` | Connection URL to the Milvus server  | `http://localhost:19530`                                  |
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

| Property     | Description                | Example Value                                             |
|--------------|----------------------------|-----------------------------------------------------------|
| `name`       | Name of the Neo4j configuration    | `semantic_neo4j_db`                                       |
| `storage_type`| Type of storage used       | `graph`                                                   |
| `config.db_name` | Name of the Neo4j database | `neo4j`                                                   |
| `config.url` | Connection URL to the Neo4j server  | `bolt://localhost:7687`                                   |
| `config.username` | Username for Neo4j server | `neo4j`                                                   |
| `config.password` | Password for Neo4j server | `password_neo`                                            |

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
