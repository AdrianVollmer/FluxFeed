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

## Features (Planned)

- ✅ Server-side rendering with Askama templates
- ✅ Modern UI with Tailwind CSS
- ✅ HTMX for snappy interactions
- 🚧 Add/remove RSS feeds
- 🚧 Auto-fetch and display articles
- 🚧 Mark articles as read/unread
- 🚧 Full-text search with FTS5
- 🚧 Resource-efficient polling with conditional GET

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

### Project Structure

```
/workspace/
├── src/
│   ├── api/              # HTTP routes & handlers
│   ├── domain/           # Business logic (to be added)
│   ├── infrastructure/   # Database, RSS fetching
│   ├── web/
│   │   └── templates/    # Askama templates
│   ├── config.rs         # Configuration
│   └── main.rs           # Entry point
├── migrations/           # SQLx database migrations
├── static/
│   ├── css/             # Tailwind output
│   └── js/              # HTMX library
└── tests/               # Integration tests (to be added)
```

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

## Phase 1: Foundation ✅

- [x] Cargo workspace with all dependencies
- [x] Database schema (feeds, articles, FTS5)
- [x] Axum server with health endpoint
- [x] Tailwind CSS build pipeline
- [x] Base templates with HTMX
- [x] SQLite setup with WAL mode

## Next Steps (Phase 2: Feed Management)

- [ ] Feed CRUD operations
- [ ] Feed list UI
- [ ] Add feed form
- [ ] Delete feed functionality

## License

MIT

## Contributors

See CONTRIBUTING.md for development guidelines.
