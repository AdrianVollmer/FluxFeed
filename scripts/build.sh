#!/usr/bin/env bash
#
# Build script for FluxFeed
#
# Usage:
#   ./scripts/build.sh [target]
#
# Targets:
#   all (default) - Build everything (frontend + rust)
#   frontend      - Build all frontend assets (css + js + icons)
#   css           - Build Tailwind CSS only
#   js            - Build TypeScript only
#   icons         - Generate PNG icons from SVG
#   rust          - Build Rust application (debug)
#   rust-release  - Build Rust application (release)
#   docker        - Build Docker image
#   clean         - Remove all build artifacts
#
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get script directory for consistent paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if npm dependencies are installed
check_npm() {
    if [ ! -d "node_modules" ]; then
        log_info "Installing npm dependencies..."
        npm ci
    fi
}

build_css() {
    log_info "Building Tailwind CSS..."
    check_npm
    npm run build:css
}

build_js() {
    log_info "Building TypeScript..."
    check_npm
    npm run build:ts
}

build_icons() {
    log_info "Generating PNG icons..."
    check_npm
    npm run generate:icons
}

build_frontend() {
    log_info "Building all frontend assets..."
    check_npm
    build_css
    build_js
    build_icons
}

build_rust() {
    log_info "Building Rust application (debug)..."
    # Ensure frontend is built first (Rust needs manifest.json at compile time)
    if [ ! -f "static/js/dist/manifest.json" ]; then
        log_warn "manifest.json not found, building frontend first..."
        build_frontend
    fi
    cargo build
}

build_rust_release() {
    log_info "Building Rust application (release)..."
    # Ensure frontend is built first
    if [ ! -f "static/js/dist/manifest.json" ]; then
        log_warn "manifest.json not found, building frontend first..."
        build_frontend
    fi
    cargo build --release
}

build_docker() {
    log_info "Building Docker image..."
    docker build -t fluxfeed:latest .
}

build_all() {
    log_info "Building everything..."
    build_frontend
    build_rust
    log_info "Build complete!"
}

clean() {
    log_info "Cleaning build artifacts..."

    # Frontend artifacts
    rm -rf static/css/tailwind.css
    rm -rf static/js/dist
    rm -rf static/icons

    # Rust artifacts
    cargo clean 2>/dev/null || true

    log_info "Clean complete!"
}

# Parse command
TARGET="${1:-all}"

case "$TARGET" in
    all)
        build_all
        ;;
    frontend)
        build_frontend
        ;;
    css)
        build_css
        ;;
    js)
        build_js
        ;;
    icons)
        build_icons
        ;;
    rust)
        build_rust
        ;;
    rust-release)
        build_rust_release
        ;;
    docker)
        build_docker
        ;;
    clean)
        clean
        ;;
    *)
        log_error "Unknown target: $TARGET"
        echo ""
        echo "Available targets:"
        echo "  all (default) - Build everything (frontend + rust)"
        echo "  frontend      - Build all frontend assets"
        echo "  css           - Build Tailwind CSS only"
        echo "  js            - Build TypeScript only"
        echo "  icons         - Generate PNG icons"
        echo "  rust          - Build Rust application (debug)"
        echo "  rust-release  - Build Rust application (release)"
        echo "  docker        - Build Docker image"
        echo "  clean         - Remove all build artifacts"
        exit 1
        ;;
esac
