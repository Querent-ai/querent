# Stage 1: Build Stage
FROM rust:bullseye AS bin-builder

ARG CARGO_PROFILE=release
ARG QUESTER_COMMIT_DATE
ARG QUESTER_COMMIT_HASH
ARG QUESTER_COMMIT_TAGS

ENV QUESTER_COMMIT_DATE=$QUESTER_COMMIT_DATE
ENV QUESTER_COMMIT_HASH=$QUESTER_COMMIT_HASH
ENV QUESTER_COMMIT_TAGS=$QUESTER_COMMIT_TAGS

RUN apt-get update \
    && apt-get install -y ca-certificates clang cmake libssl-dev llvm protobuf-compiler python3-dev \
    && rm -rf /var/lib/apt/lists/*

RUN apt-get update \
    && apt-get install -y ca-certificates libssl1.1 python3 python3-pip python3-dev tesseract-ocr \
    && rm -rf /var/lib/apt/lists/*
    
# Required by tonic
RUN rustup component add rustfmt

COPY quester /quester
COPY config /config
WORKDIR /quester

RUN echo "Building workspace with feature(s) '--all-features' and profile '$CARGO_PROFILE'" \
    && cargo build --all-features $(test "$CARGO_PROFILE" = "release" && echo "--release") \
    && echo "Copying binaries to /quester/bin" \
    && mkdir -p /quester/bin \
    && find target/$CARGO_PROFILE -maxdepth 1 -perm /a+x -type f -exec mv {} /quester/bin \;

# Stage 2: Runtime Stage
FROM debian:bullseye-slim AS quester

LABEL org.opencontainers.image.title="Querent"
LABEL maintainer="Querent, AI <hello@querent.xyz>"
LABEL org.opencontainers.image.vendor="Querent, AI"
LABEL org.opencontainers.image.licenses="BSL 1.1"


WORKDIR /quester
RUN mkdir config quester_data

# Copy binaries and configuration from the build stage
COPY --from=bin-builder /quester/bin/querent /usr/local/bin/querent
RUN chmod +x /usr/local/bin/querent
COPY --from=bin-builder /config/querent.config.yaml /quester/config/querent.config.yaml

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
