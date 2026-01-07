mod api;
mod config;
mod domain;
mod infrastructure;
mod web;

/// Returns the FluxFeed user agent string with the current version
///
/// Format: "FluxFeed/X.Y.Z"
///
/// The version is read from Cargo.toml at compile time, ensuring it's
/// always in sync with the package version.
pub fn user_agent() -> String {
    format!("FluxFeed/{}", env!("CARGO_PKG_VERSION"))
}

use api::feeds::AppState;
use axum::{
    middleware,
    response::Redirect,
    routing::{delete, get, post, put},
    Router,
};
use config::Config;
use infrastructure::csrf::csrf_middleware;
use infrastructure::database::setup_database;
use infrastructure::security_headers::security_headers_middleware;
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

async fn index() -> Redirect {
    Redirect::to("/articles")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    // Default to info level in production; use RUST_LOG env var to override
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fluxfeed=info,tower_http=info".into()),
        )
        .init();

    // Load configuration
    let config = Config::from_env()?;

    // Setup database
    tracing::info!("Connecting to database: {}", config.database_url);
    let db_pool = setup_database(&config.database_url).await?;

    // Run migrations
    tracing::info!("Running database migrations");
    sqlx::migrate!().run(&db_pool).await?;
    tracing::info!("Migrations complete");

    // Create shared application state
    let state = AppState {
        db_pool: db_pool.clone(),
    };

    // Start background scheduler for RSS fetching
    tracing::info!("Starting RSS feed scheduler");
    let _scheduler = infrastructure::scheduler::start_scheduler(state.clone()).await?;

    // Build router
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(api::health::check))
        .route(
            "/feeds",
            get(api::feeds::list_feeds).post(api::feeds::create_feed),
        )
        .route("/feeds/new", get(api::feeds::show_feed_form))
        .route("/feeds/import/form", get(api::feeds::show_import_form))
        .route("/feeds/import", post(api::feeds::import_feeds))
        .route(
            "/feeds/:id",
            get(api::feeds::show_feed).delete(api::feeds::delete_feed),
        )
        .route("/feeds/:id/fetch", post(api::feeds::fetch_feed))
        .route("/feeds/:id/edit", get(api::feeds::show_edit_feed_form))
        .route("/feeds/:id/update", post(api::feeds::update_feed))
        .route("/feeds/:id/group", put(api::groups::assign_feed_to_group))
        // Group routes
        .route(
            "/groups",
            get(api::groups::list_groups).post(api::groups::create_group),
        )
        .route("/groups/new", get(api::groups::show_new_group_form))
        .route(
            "/groups/assign-feed/:feed_id",
            get(api::groups::show_assign_feed_form),
        )
        .route(
            "/groups/:id",
            delete(api::groups::delete_group).put(api::groups::update_group),
        )
        .route("/groups/:id/edit", get(api::groups::show_edit_group_form))
        .route("/groups/:id/parent", put(api::groups::move_group))
        .route("/articles", get(api::articles::list_articles))
        .route("/articles/search", get(api::articles::search_articles))
        .route(
            "/articles/filter-modal",
            get(api::groups::show_feed_filter_modal),
        )
        .route(
            "/articles/:id/toggle-read",
            post(api::articles::toggle_read_status),
        )
        .route(
            "/articles/:id/toggle-read-compact",
            post(api::articles::toggle_read_status_compact),
        )
        .route(
            "/articles/:id/mark-read",
            post(api::articles::mark_read_status),
        )
        .route(
            "/articles/:id/mark-read-compact",
            post(api::articles::mark_read_status_compact),
        )
        .route(
            "/articles/:id/toggle-starred",
            post(api::articles::toggle_starred_status),
        )
        .route(
            "/articles/:id/toggle-starred-compact",
            post(api::articles::toggle_starred_status_compact),
        )
        .route(
            "/articles/mark-all-read",
            post(api::articles::mark_all_read),
        )
        .route("/articles/:id/reader", get(api::reader::show_reader_mode))
        .route("/logs", get(api::logs::list_logs))
        .route("/api/fetch", post(api::manual_fetch::trigger_fetch))
        .nest_service("/static", ServeDir::new("static"))
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(middleware::from_fn(csrf_middleware))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let bind_addr = format!("{}:{}", config.host, config.port);
    tracing::info!("FluxFeed server listening on http://{}", bind_addr);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
