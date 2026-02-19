# Multi-stage Dockerfile for Sentinel
# Stage 1: Builder
FROM rust:bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates
COPY .sqlx ./.sqlx

# Build release binary
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /opt/sentinel

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create directories
RUN mkdir -p /opt/sentinel /data /opt/sentinel/scripts

# Copy binary from builder
COPY --from=builder /app/target/release/sentinel /opt/sentinel/sentinel

# Copy migrations and templates
COPY --from=builder /app/migrations /opt/sentinel/migrations
COPY --from=builder /app/templates /opt/sentinel/templates

# Expose port
EXPOSE 3000

# Set environment
ENV RUST_LOG=sentinel=info,tower_http=debug

# Run the application
CMD ["/opt/sentinel/sentinel"]
