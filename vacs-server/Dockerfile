ARG RUST_VERSION=1.89.0
ARG DEBIAN_VERSION=trixie
FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} AS chef

WORKDIR /usr/src/vacs-server
RUN cargo install cargo-chef --locked

FROM chef AS planner

COPY . .
RUN cargo chef prepare --bin vacs-server --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /usr/src/vacs-server/recipe.json recipe.json
RUN cargo chef cook --release --bin vacs-server --recipe-path recipe.json

COPY . .

RUN cargo build --release --bin vacs-server

FROM debian:${DEBIAN_VERSION}-slim AS runtime

LABEL org.opencontainers.image.source=https://github.com/MorpheusXAUT/vacs
LABEL org.opencontainers.image.description="VATSIM ATC Comm System - Signaling Server"
LABEL org.opencontainers.image.licenses=MIT

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      ca-certificates \
      tzdata \
      curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/vacs-server/target/release/vacs-server /usr/local/bin/vacs-server

VOLUME ["/etc/vacs-server"]

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
    CMD curl -fsS http://127.0.0.1:3000/health || exit 1

ENTRYPOINT ["/usr/local/bin/vacs-server"]
