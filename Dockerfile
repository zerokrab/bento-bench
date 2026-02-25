# syntax=docker/dockerfile:1
ARG RUST_IMG=rust:1.91-bookworm

FROM ${RUST_IMG} AS chef

ARG DEBIAN_FRONTEND=noninteractive
ENV TZ="America/Los_Angeles"

RUN apt-get -qq update && apt-get install -y -q \
    openssl libssl-dev pkg-config curl clang git \
    build-essential openssh-client unzip

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN cargo install cargo-chef --locked

# Install RISC0 early for better caching (needed by build scripts during cook)
ENV RISC0_HOME=/usr/local/risc0
ENV PATH="/root/.cargo/bin:${PATH}"

RUN curl -L https://risczero.com/install | bash && \
    /root/.risc0/bin/rzup install && \
    rm -rf /tmp/* /var/tmp/*


FROM chef AS planner

WORKDIR /src
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder

WORKDIR /src
COPY --from=planner /src/recipe.json recipe.json
# Build dependencies only — this layer is cached unless Cargo.toml/Cargo.lock changes
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release -p bento-bench --bin bento-bench


FROM debian:bookworm-slim AS runtime

RUN apt-get update -q -y \
    && apt-get install -q -y ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/bento-bench /app/bento-bench
COPY --from=builder /usr/local/risc0 /usr/local/risc0
COPY scripts/docker-run-benchmarks.sh /app/docker-run-benchmarks.sh

VOLUME ["/data"]
VOLUME ["/manifest.json"]

ENTRYPOINT ["/app/docker-run-benchmarks.sh"]
