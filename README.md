# FluxFeed

A modern, resource-efficient RSS feed reader built with Rust.

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

- Rust (1.70+)
- Node.js (for Tailwind CSS)
- pkg-config and libssl-dev (for OpenSSL)

### Setup

1. **Install dependencies:**
```bash
npm install
```

2. **Build Tailwind CSS:**
```bash
npm run build:css
# Or watch for changes:
npm run watch:css
```

3. **Build and run:**
```bash
cargo build
cargo run
```

The server will start on http://localhost:3000

### Database

Migrations are embedded in the binary and run automatically on startup.

To manually run migrations:
```bash
sqlx migrate run
```

### Environment Variables

Copy `.env.example` to `.env` and customize:

```bash
DATABASE_URL=sqlite://fluxfeed.db
PORT=3000
RUST_LOG=info
```

## License

MIT

## Contributors

See CONTRIBUTING.md for development guidelines.
