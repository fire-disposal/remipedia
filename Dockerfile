# ============================================
# Stage 1: Build
# ============================================
FROM rust:1.94.0-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN cargo build --release && rm -rf src

COPY src ./src
COPY migrations ./migrations
COPY static ./static
RUN touch src/main.rs src/lib.rs && cargo build --release

# ============================================
# Stage 2: Runtime
# ============================================
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 curl && rm -rf /var/lib/apt/lists/*
RUN useradd -r -s /bin/false appuser

COPY --from=builder /app/target/release/remipedia /app/remipedia
COPY --from=builder /app/migrations /app/migrations
COPY config /app/config

RUN chown -R appuser:appuser /app
USER appuser

EXPOSE 8000

ENV RUST_LOG=info
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

CMD ["./remipedia"]