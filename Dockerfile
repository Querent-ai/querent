# Use a more specific base image if possible
FROM debian:bookworm-slim AS querent_runtime

LABEL org.opencontainers.image.title="Querent" \
      maintainer="Querent, AI <contact@querent.xyz>" \
      org.opencontainers.image.vendor="Querent, AI" \
      org.opencontainers.image.licenses="BSL 1.1"

WORKDIR /rian

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
    && mkdir -p /rian/config /rian/querent_data \
    && mv rian-*/rian /usr/local/bin/ \
    && mv rian-*/config/querent.config.yaml /rian/config/ \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* rian-*

# Install tesseract-ocr and its library
RUN apt-get update && apt-get install -y \
    tesseract-ocr \
    tesseract-ocr-eng \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN chmod +x /usr/local/bin/rian && \
    rian --version && \
    echo "Querent RIAN is ready to run!"

COPY scripts/docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENV QUERENT_CONFIG=/rian/config/querent.config.yaml \
    QUERENT_DATA_DIR=/rian/querent_data \
    QUERENT_LISTEN_ADDRESS=0.0.0.0 \
    PYTHONIOENCODING=utf-8 \
    LANG=C.UTF-8

ENTRYPOINT ["entrypoint.sh"]
