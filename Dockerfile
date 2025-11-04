# syntax=docker/dockerfile:1
ARG RUST_IMG=rust:1.89-bookworm

FROM ${RUST_IMG} AS rust-builder

ARG DEBIAN_FRONTEND=noninteractive
ENV TZ="America/Los_Angeles"

RUN apt-get -qq update && apt-get install -y -q \
    openssl libssl-dev pkg-config curl clang git \
    build-essential openssh-client unzip

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# # Install RISC0 and groth16 component early for better caching
ENV RISC0_HOME=/usr/local/risc0
ENV PATH="/root/.cargo/bin:${PATH}"

# # Install RISC0 and groth16 component - this layer will be cached unless RISC0_HOME changes
RUN curl -L https://risczero.com/install | bash && \
    /root/.risc0/bin/rzup install && \
    # Clean up any temporary files to reduce image size
    rm -rf /tmp/* /var/tmp/*

FROM rust-builder AS builder

WORKDIR /src/
COPY . .

RUN cargo build --release -p bencher --bin bencher

FROM debian:bookworm-slim AS runtime

RUN apt-get update -q -y \
    && apt-get install -q -y ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/bencher /app/bencher
COPY --from=builder /usr/local/risc0 /usr/local/risc0
COPY scripts/docker-run-benchmarks.sh /app/docker-run-benchmarks.sh

ENTRYPOINT ["/app/docker-run-benchmarks.sh"]