# Multi-stage build for minimal final image

# Stage 1: Build frontend assets (CSS, JS, and icons)
FROM node:20-alpine AS frontend-builder
WORKDIR /build
COPY . .

RUN apk add --no-cache bash fontconfig
RUN npm ci
RUN ./scripts/build.sh frontend

# Stage 2: Build Rust application
FROM rust:1.92-alpine AS builder
WORKDIR /build
COPY . .

# Install build dependencies for musl
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    perl \
    make \
    openssl-dev \
    openssl-libs-static

# Create dummy manifest.json for dependency caching (include_str! needs it)
RUN mkdir -p static/js/dist && echo '{}' > static/js/dist/manifest.json

# Create a dummy main.rs to build dependencies (for caching)
RUN mv src src-real && mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    SQLX_OFFLINE=true cargo build --release && \
    rm -rf src && \
    mv src-real src

# Copy real manifest from frontend-builder
COPY --from=frontend-builder /build/static/js/dist ./static/js/dist

# Build the application with real source
RUN SQLX_OFFLINE=true cargo build --release

# Stage 3: Runtime image
FROM alpine:3.21
WORKDIR /app

# Install runtime dependencies (wget is included in busybox)
RUN apk add --no-cache \
    ca-certificates \
    tzdata

# Create a non-root user
RUN adduser -D -u 1000 fluxfeed && \
    mkdir -p /app/data && \
    chown -R fluxfeed:fluxfeed /app

# Copy binary from builder
COPY --from=builder /build/target/release/fluxfeed /app/fluxfeed

# Copy static files from frontend-builder (includes compiled CSS, JS, and icons)
COPY --from=frontend-builder --chown=fluxfeed:fluxfeed /build/static /app/static

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
    CMD wget -q --spider http://localhost:${PORT:-3000}/health || exit 1

# Run the application
CMD ["/app/fluxfeed"]
