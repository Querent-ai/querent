---
title: Quickstart
sidebar_position: 1
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

In this quick start guide, we will install Querent, create an index, add documents and finally execute search queries. All the Querent commands used in this guide are documented [in the CLI reference documentation](/docs/reference/cli.md).

## Install Querent using Querent installer

The Querent installer automatically picks the correct binary archive for your environment and then downloads and unpacks it in your working directory.
This method works only for [some OS/architectures](installation.md#download), and you will also need to install some [external dependencies](installation.md#note-on-external-dependencies).

```bash
pip3 install querent
curl -L https://install.querent.xyz | sh
```

```bash
cd ./rian-v*/
./rian --version
```

You can now move this executable directory wherever sensible for your environment and possibly add it to your `PATH` environment.

## Use Querent's Docker image

You can also pull and run the Querent binary in an isolated Docker container.

Here is an example docker compose environment:

```yaml
version: '3'
services:
  etcd:
    container_name: milvus-etcd
    image: quay.io/coreos/etcd:v3.5.5
    environment:
      - ETCD_AUTO_COMPACTION_MODE=revision
      - ETCD_AUTO_COMPACTION_RETENTION=1000
      - ETCD_QUOTA_BACKEND_BYTES=4294967296
      - ETCD_SNAPSHOT_COUNT=50000
    volumes:
      - ${DOCKER_VOLUME_DIRECTORY:-.}/volumes/etcd:/etcd
    command: etcd -advertise-client-urls=http://127.0.0.1:2379 -listen-client-urls http://0.0.0.0:2379 --data-dir /etcd
    healthcheck:
      test: ["CMD", "etcdctl", "endpoint", "health"]
      interval: 30s
      timeout: 20s
      retries: 3
    networks:
      - querent

  minio:
    container_name: milvus-minio
    image: minio/minio:RELEASE.2023-03-20T20-16-18Z
    environment:
      MINIO_ACCESS_KEY: minioadmin
      MINIO_SECRET_KEY: minioadmin
    ports:
      - "9001:9001"
      - "9000:9000"
    volumes:
      - ${DOCKER_VOLUME_DIRECTORY:-.}/volumes/minio:/minio_data
    command: minio server /minio_data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 30s
      timeout: 20s
      retries: 3
    networks:
      - querent

  milvus:
    container_name: milvus-standalone
    image: milvusdb/milvus:v2.3.4
    command: ["milvus", "run", "standalone"]
    security_opt:
    - seccomp:unconfined
    environment:
      ETCD_ENDPOINTS: etcd:2379
      MINIO_ADDRESS: minio:9000
    volumes:
      - ${DOCKER_VOLUME_DIRECTORY:-.}/volumes/milvus:/var/lib/milvus
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9091/healthz"]
      interval: 30s
      start_period: 90s
      timeout: 20s
      retries: 3
    ports:
      - "19530:19530"
      - "9091:9091"
    depends_on:
      - "etcd"
      - "minio"
    networks:
      - querent

  postgres:
    image: postgres
    environment:
      - POSTGRES_USER=querent
      - POSTGRES_PASSWORD=querent
      - POSTGRES_DB=querent_test
    ports:
      - "5432:5432"
    networks:
      - querent
    healthcheck:
      test: ["CMD-SHELL", "pg_isready", "-d", "querent_test"]
      interval: 30s
      timeout: 60s
      retries: 5
      start_period: 80s

  neo4j:
    image: neo4j
    environment:
      - NEO4J_AUTH=neo4j/password_neo
    ports:
      - "7474:7474"
      - "7687:7687"
    networks:
      - querent
    healthcheck:
      test: wget http://localhost:7474 || exit 1
      interval: 1s
      timeout: 10s
      retries: 20
      start_period: 3s
  
  querent:
    image: querent/querent:v0.0.3-rc9
    ports:
      - "1111:1111"
      - "2222:2222"
      - "3333:3333"
    depends_on:
      postgres:
        condition: service_healthy
      neo4j:
        condition: service_healthy
      etcd:
        condition: service_healthy
      minio:
        condition: service_healthy
      milvus:
        condition: service_healthy
    volumes:
      - ./config:/external-config
    environment:
      QUERENT_NODE_CONFIG: /external-config/querent.config.docker.yaml
      PYTHONIOENCODING: utf-8
      LANG: C.UTF-8
    networks:
      - querent

networks:
  querent:

```

```bash
docker-compose up
```

## Start Querent server using standalone binary

<Tabs>

<TabItem value="cli" label="CLI">

```bash
./rian serve --config ./config/querent.config.yaml
```

</TabItem>

<TabItem value="docker" label="Docker">

```bash
docker run --rm -v $(pwd)/querent_data:/querent/querent_data -p 127.0.0.1:7280:7280 querent/querent env QUERENT_NODE_CONFIG=/path/to/querent.config.yaml
```

</TabItem>

</Tabs>

Tips: you can use the environment variable `RUST_LOG` to control querent verbosity.

Check it's working by browsing the [UI at http://localhost:111](http://localhost:1111) or do a simple GET with cURL:

```bash
curl http://localhost:7280/api/v1/version
```

Here are the links you check:

- [Swagger API documentation](http://localhost:1111/swagger-ui)

- [Querent Dashboard](http://localhost:1111)
