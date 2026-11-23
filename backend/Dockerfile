# ==============================================================================
# Stage 1: Build binary using latest stable Rust
# ==============================================================================
FROM rust:latest AS builder
WORKDIR /app

# Copy dependency manifests and source files
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# Compile optimized release binary
RUN cargo build --release

# ==============================================================================
# Stage 2: Minimal runtime with FFmpeg & SSL certificates
# ==============================================================================
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary and database migrations
COPY --from=builder /app/target/release/nyaalist-backend /app/nyaalist-backend
COPY --from=builder /app/migrations /app/migrations

# Ensure default media and storage directories exist
RUN mkdir -p /data/downloads /data/library /data/covers /data/temp_encode

ENV RUST_LOG=info,nyaalist=debug,tower_http=info
ENV PORT=5041
ENV HOST=0.0.0.0
ENV DOWNLOADS_DIR=/data/downloads
ENV LIBRARY_DIR=/data/library
ENV COVERS_DIR=/data/covers
ENV TEMP_ENCODE_DIR=/data/temp_encode

EXPOSE 5041

ENTRYPOINT ["/app/nyaalist-backend"]
