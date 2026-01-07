# Multi-stage build for minimal final image

# Stage 1: Build frontend assets (CSS and JS)
FROM node:20-slim AS frontend-builder
WORKDIR /build
COPY package*.json ./
RUN npm ci
COPY static/css/input.css ./static/css/
COPY static/js/ts ./static/js/ts
COPY scripts/build-ts.js ./scripts/
COPY tailwind.config.js tsconfig.json ./
COPY src/web/templates ./src/web/templates
RUN npm run build

# Stage 2: Build Rust application
FROM rust:1.92-bookworm AS builder
WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    perl \
    make \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and SQLx metadata
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY --from=frontend-builder /build/static/js/manifest.json ./static/js/

# Create a dummy main.rs to build dependencies (for caching)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    SQLX_OFFLINE=true cargo build --release && \
    rm -rf src

# Copy source code and templates
COPY src ./src
COPY migrations ./migrations
COPY askama.toml ./

# Build the application
RUN SQLX_OFFLINE=true cargo build --release

# Stage 3: Runtime image
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies (including tzdata for timezone support)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libcurl4 \
    curl \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 fluxfeed && \
    mkdir -p /app/data && \
    chown -R fluxfeed:fluxfeed /app

# Copy binary from builder
COPY --from=builder /build/target/release/fluxfeed /app/fluxfeed

# Copy static files (base assets: icons, htmx, favicon, etc.)
COPY --chown=fluxfeed:fluxfeed static /app/static

# Copy compiled CSS and JS from frontend-builder
COPY --from=frontend-builder --chown=fluxfeed:fluxfeed /build/static/css/tailwind.css /app/static/css/
COPY --from=frontend-builder --chown=fluxfeed:fluxfeed /build/static/js/dist /app/static/js/dist
COPY --from=frontend-builder --chown=fluxfeed:fluxfeed /build/static/js/manifest.json /app/static/js/

# Switch to non-root user
USER fluxfeed

# Set environment variables with defaults
ENV DATABASE_URL=sqlite:///app/data/fluxfeed.db \
    PORT=3000 \
    HOST=0.0.0.0 \
    RUST_LOG=info \
    TZ=UTC

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${PORT:-3000}/health || exit 1

# Run the application
CMD ["/app/fluxfeed"]
