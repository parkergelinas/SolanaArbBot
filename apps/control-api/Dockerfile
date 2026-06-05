# Build from repository root:
#   docker build -f apps/control-api/Dockerfile -t solana-control-api .

FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p control-api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/control-api /usr/local/bin/control-api

ENV CONTROL_API_HOST=0.0.0.0
ENV CONTROL_API_PORT=3001
ENV DEPLOY_ENV=production
ENV RUST_LOG=info

EXPOSE 3001

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -fsS http://127.0.0.1:3001/api/health || exit 1

CMD ["control-api"]
