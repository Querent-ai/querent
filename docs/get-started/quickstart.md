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
