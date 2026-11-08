# FluxFeed task runner. Run `just` to list all recipes.

set shell := ["bash", "-euo", "pipefail", "-c"]

# List available recipes
default:
    @just --list

# Build everything (frontend + rust debug)
build:
    ./scripts/build.sh all

# Build all frontend assets (css + js + icons)
frontend:
    ./scripts/build.sh frontend

# Build Tailwind CSS only
css:
    ./scripts/build.sh css

# Build TypeScript only
js:
    ./scripts/build.sh js

# Generate PNG icons from SVG
icons:
    ./scripts/build.sh icons

# Build Rust application (debug)
rust:
    ./scripts/build.sh rust

# Build Rust application (release)
rust-release:
    ./scripts/build.sh rust-release

# Build Docker image
docker:
    ./scripts/build.sh docker

# Remove all build artifacts
clean:
    ./scripts/build.sh clean

# Watch CSS and TypeScript for changes
watch:
    npm run watch

# Type-check TypeScript
typecheck:
    npm run typecheck

# Run the debug binary
run: rust
    ./target/debug/fluxfeed

# Format Rust code
fmt:
    cargo fmt

# Lint Rust code
lint:
    cargo clippy --all-targets -- -D warnings

# Run the Rust test suite
test:
    cargo test

# Bump the project version, commit, and tag (e.g. `just bump-version 0.2.7`)
bump-version version:
    ./scripts/bump-version.sh {{version}}

# Create a stress-test SQLite database with 1000+ feeds and 100k+ articles
stress-test:
    ./scripts/stress-test.py

# Take desktop/mobile screenshots of the articles page for docs
screenshots:
    ./scripts/take_screenshots.py

# Take screenshots of the compact list view
screenshots-list:
    ./scripts/take_list_screenshots.py

# Take light/dark screenshots of the fullscreen reader view
screenshots-fullscreen:
    ./scripts/take_fullscreen_screenshots.py
