# Multi-stage build for minimal final image

# Stage 1: Build Tailwind CSS
FROM node:20-slim AS css-builder
WORKDIR /build
COPY package*.json ./
RUN npm ci
COPY static/css/input.css ./static/css/
COPY tailwind.config.js ./
RUN npm run build:css

# Stage 2: Build Rust application
FROM rust:1.92-slim AS builder
WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies (for caching)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code and templates
COPY src ./src
COPY migrations ./migrations
COPY askama.toml ./

# Build the application
RUN cargo build --release

# Stage 3: Runtime image
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 fluxfeed && \
    mkdir -p /app/data && \
    chown -R fluxfeed:fluxfeed /app

# Copy binary from builder
COPY --from=builder /build/target/release/fluxfeed /app/fluxfeed

# Copy static files
COPY --chown=fluxfeed:fluxfeed static /app/static

# Copy compiled CSS from css-builder
COPY --from=css-builder --chown=fluxfeed:fluxfeed /build/static/css/tailwind.css /app/static/css/

# Switch to non-root user
USER fluxfeed

# Set environment variables with defaults
ENV DATABASE_URL=sqlite:///app/data/fluxfeed.db \
    PORT=3000 \
    HOST=0.0.0.0 \
    RUST_LOG=info

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/app/fluxfeed", "--version"] || exit 1

# Run the application
CMD ["/app/fluxfeed"]
