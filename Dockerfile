# Use a more specific base image if possible
FROM debian:bookworm-slim AS querent_runtime

LABEL org.opencontainers.image.title="Querent" \
      maintainer="Querent, AI <contact@querent.xyz>" \
      org.opencontainers.image.vendor="Querent, AI" \
      org.opencontainers.image.licenses="BSL 1.1"

WORKDIR /querent


RUN apt-get -y update \
    && apt-get -y install ca-certificates \
                          libssl3 \
    && rm -rf /var/lib/apt/lists/*
    
# Combine update, install, and cleanup steps to reduce layers and ensure cleanup is effective
RUN apt-get update && apt-get install -y \
    curl \
    bc \
    tar \
    gzip \
    && curl -L https://install.querent.xyz/install.sh | sh \
    && mkdir -p /querent/config /querent/querent_data \
    && mv querent-*/querent /usr/local/bin/ \
    && mv querent-*/config/querent.config.yaml /querent/config/ \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* querent-*

RUN chmod +x /usr/local/bin/querent && \
    querent --version && \
    echo "Querent is ready to run!"

COPY scripts/docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENV QUERENT_CONFIG=/querent/config/querent.config.yaml \
    QUERENT_DATA_DIR=/querent/querent_data \
    QUERENT_LISTEN_ADDRESS=0.0.0.0 \
    PYTHONIOENCODING=utf-8 \
    LANG=C.UTF-8

ENTRYPOINT ["entrypoint.sh"]