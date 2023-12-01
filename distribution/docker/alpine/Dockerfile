FROM alpine:3 AS builder

COPY quester-*-unknown-linux-musl.tar.gz ./
RUN tar -xzf quester-*-$(cat /etc/apk/arch)-unknown-linux-musl.tar.gz
RUN mv ./quester-*/* ./
RUN chmod 744 ./quester


FROM alpine:3

LABEL org.opencontainers.image.title="Quester"
LABEL maintainer="Quester, Inc. <hello@quester.io>"
LABEL org.opencontainers.image.vendor="Quester, Inc."
LABEL org.opencontainers.image.licenses="AGPL-3.0"

WORKDIR /quester
RUN mkdir config qwdata
COPY --from=builder /quester /usr/local/bin/
COPY --from=builder /config/quester.yaml /quester/config/quester.yaml

ENV QW_CONFIG=/quester/config/quester.yaml
ENV QW_DATA_DIR=/quester/qwdata
ENV QW_LISTEN_ADDRESS=0.0.0.0

ENTRYPOINT ["/usr/local/bin/quester"]
