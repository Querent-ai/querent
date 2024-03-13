# Use a more specific base image if possible
FROM ubuntu:22.04 AS querent_runtime

LABEL org.opencontainers.image.title="Querent" \
      maintainer="Querent, AI <contact@querent.xyz>" \
      org.opencontainers.image.vendor="Querent, AI" \
      org.opencontainers.image.licenses="BSL 1.1"

WORKDIR /quester

# Combine update, install, and cleanup steps to reduce layers and ensure cleanup is effective
RUN apt-get update && apt-get install -y \
    curl \
    bc \
    tar \
    gzip \
    python3 \
    python3-pip \
    tesseract-ocr \
    ffmpeg \
    && curl -L https://install.querent.xyz/install.sh | sh \
    && mkdir -p /quester/config /quester/querent_data \
    && mv querent-*/querent /usr/local/bin/ \
    && mv querent-*/config/querent.config.yaml /quester/config/ \
    && pip3 install --no-cache-dir torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu \
    && pip3 install querent \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* querent-*

RUN chmod +x /usr/local/bin/querent && \
    querent --version && \
    echo "Querent is ready to run!"

COPY scripts/docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENV QUERENT_CONFIG=/quester/config/querent.config.yaml \
    QUERENT_DATA_DIR=/quester/querent_data \
    QUERENT_LISTEN_ADDRESS=0.0.0.0 \
    PYTHONIOENCODING=utf-8 \
    LANG=C.UTF-8

ENTRYPOINT ["entrypoint.sh"]
