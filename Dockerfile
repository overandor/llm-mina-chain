# Multi-stage build for llm-mina-chain
# Usage:
#   docker build -t llm-mina-chain .
#   docker run -p 8000:8000 -e SOLANA_RPC_ENDPOINT=https://api.mainnet-beta.solana.com llm-mina-chain

# ---- Build stage ----
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests first for layer caching
COPY rust/Cargo.toml rust/Cargo.lock ./
COPY rust/src ./src

# Build release binaries
RUN cargo build --release --bin solana-agent-server --bin solana-agent-cli --bin semantic-runtime

# ---- Runtime stage ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/solana-agent-server /usr/local/bin/
COPY --from=builder /app/target/release/solana-agent-cli /usr/local/bin/
COPY --from=builder /app/target/release/semantic-runtime /usr/local/bin/

# Default: run the semantic runtime server
ENV BIND_ADDR=0.0.0.0:8000
ENV SOLANA_RPC_ENDPOINT=https://api.mainnet-beta.solana.com
EXPOSE 8000

CMD ["solana-agent-server"]
