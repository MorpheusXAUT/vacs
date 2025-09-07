ARG RUST_VERSION=1.89.0
ARG DEBIAN_VERSION=trixie
FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} AS chef

WORKDIR /usr/src/vacs
RUN cargo install cargo-chef --locked

FROM chef AS planner

COPY . .
RUN cargo chef prepare --bin vacs-server --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /usr/src/vacs/recipe.json recipe.json
RUN cargo chef cook --release --bin vacs-server --recipe-path recipe.json

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/vacs/target \
    cargo build --release --bin vacs-server

FROM debian:${DEBIAN_VERSION}-slim AS runtime

LABEL org.opencontainers.image.source=https://github.com/MorpheusXAUT/vacs
LABEL org.opencontainers.image.description="VATSIM ATC Comm System - Signaling Server"
LABEL org.opencontainers.image.licenses=MIT

# Create a non-root user and group
RUN useradd -m -u 10001 -U vacs

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      ca-certificates \
      tzdata \
      curl && \
    rm -rf /var/lib/apt/lists/*

VOLUME ["/etc/vacs-server"]
RUN mkdir -p /etc/vacs-server && chown -R vacs:vacs /etc/vacs-server

COPY --from=builder /usr/src/vacs/target/release/vacs-server /usr/local/bin/vacs-server

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
    CMD curl -fsS http://127.0.0.1:3000/health || exit 1

USER vacs
ENTRYPOINT ["/usr/local/bin/vacs-server"]
