# Runtime Stage
FROM debian:bullseye-slim AS quester

LABEL org.opencontainers.image.title="Querent"
LABEL maintainer="Querent, AI <hello@querent.xyz>"
LABEL org.opencontainers.image.vendor="Querent, AI"
LABEL org.opencontainers.image.licenses="BSL 1.1"


WORKDIR /quester
RUN mkdir config quester_data

# Copy binaries from quester/target/release
COPY quester/target/release/querent /usr/local/bin/querent
RUN chmod +x /usr/local/bin/querent
COPY config/querent.config.yaml /quester/config/querent.config.yaml

RUN apt-get update \
    && apt-get install -y ca-certificates libssl1.1 python3 python3-pip python3-dev tesseract-ocr \
    && rm -rf /var/lib/apt/lists/*

# Install querent
#RUN pip3 install querent

ENV QUESTER_CONFIG=/quester/config/querent.config.yaml
ENV QUESTER_DATA_DIR=/quester/quester_data
ENV QUESTER_LISTEN_ADDRESS=0.0.0.0

RUN querent --version \
    && echo "Querent is ready to run!"

COPY scripts/docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh
ENTRYPOINT ["entrypoint.sh"]
