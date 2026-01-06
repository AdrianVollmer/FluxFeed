# FluxFeed

A modern, resource-efficient RSS feed reader built with Rust.

The primary use case is running this program as a self-hosted Docker
instance accessed locally or via VPN. There is currently no support for
multiple users.

## Tech Stack

- **Web Framework:** Axum 0.7
- **Templates:** Askama (compile-time checked)
- **Interactivity:** HTMX
- **Database:** SQLite with SQLx
- **RSS Parser:** feed-rs
- **UI:** Tailwind CSS
- **Background Jobs:** tokio-cron-scheduler

## Development

### Prerequisites

- Rust (1.92+)
- Node.js (for Tailwind CSS)
- pkg-config and libssl-dev (for OpenSSL)

### Building

1.  **Install dependencies:**

``` bash
npm install
```

2.  **Build Tailwind CSS:**

``` bash
npm run build:css
# Or watch for changes:
npm run watch:css
```

3.  **Build and run:**

``` bash
SQLX_OFFLINE=true cargo build --release
./target/release/fluxfeed
```

The server will start on <http://localhost:3000>.

### Database

Migrations are embedded in the binary and run automatically on startup.

To manually run migrations:

``` bash
sqlx migrate run
```

### Environment Variables

Copy `.env.example` to `.env` and customize:

``` bash
DATABASE_URL=sqlite://fluxfeed.db
PORT=3000
HOST=0.0.0.0
RUST_LOG=info
```

## Docker Deployment

The easiest way to run FluxFeed is with Docker.

### Using Docker Compose (Recommended)

``` bash
docker compose up -d
```

The app will be available at <http://localhost:3000> with persistent
storage.

### Using Docker directly

There is a publicly available Docker image:
`ghcr.io/adrianvollmer/fluxfeed:latest`

To build the image:

``` bash
docker build -t fluxfeed .
```

Run the container:

``` bash
docker run --rm -d \
  -p 3000:3000 \
  -v fluxfeed-data:/app/data \
  -e DATABASE_URL=sqlite:///app/data/fluxfeed.db \
  -e PORT=3000 \
  -e HOST=0.0.0.0 \
  --name fluxfeed \
  fluxfeed
```

### Environment Variables for Docker

- `DATABASE_URL`: Path to SQLite database (default:
  `sqlite:///app/data/fluxfeed.db`)
- `PORT`: Port to listen on (default: `3000`)
- `HOST`: IP address to bind to (default: `0.0.0.0`)
- `RUST_LOG`: Log level (default: `info`)

## License

MIT

## Contributors

See CONTRIBUTING.md for development guidelines.
