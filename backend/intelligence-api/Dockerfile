# Build from repository root:
#   docker build -f backend/intelligence-api/Dockerfile -t solana-intelligence-api .

FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p intelligence-api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/intelligence-api /usr/local/bin/intelligence-api

ENV INTELLIGENCE_PORT=8090
ENV RUST_LOG=info

EXPOSE 8090

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8090/health || exit 1

CMD ["intelligence-api"]
