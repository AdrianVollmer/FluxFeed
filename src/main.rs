mod api;
mod config;
mod domain;
mod infrastructure;
mod web;

use api::feeds::AppState;
use askama::Template;
use axum::{response::Html, routing::{delete, get, post}, Router};
use config::Config;
use infrastructure::database::setup_database;
use std::net::SocketAddr;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, trace::TraceLayer,
};
use web::templates::IndexTemplate;

async fn index() -> Html<String> {
    let template = IndexTemplate;
    Html(template.render().unwrap())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    "fluxfeed=debug,tower_http=debug".into()
                }),
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
        .route("/feeds", get(api::feeds::list_feeds).post(api::feeds::create_feed))
        .route("/feeds/new", get(api::feeds::show_feed_form))
        .route("/feeds/:id", get(api::feeds::show_feed).delete(api::feeds::delete_feed))
        .route("/articles", get(api::articles::list_articles))
        .route("/articles/:id/toggle-read", post(api::articles::toggle_read_status))
        .route("/articles/:id/toggle-read-compact", post(api::articles::toggle_read_status_compact))
        .route("/articles/mark-all-read", post(api::articles::mark_all_read))
        .route("/logs", get(api::logs::list_logs))
        .route("/api/fetch", post(api::manual_fetch::trigger_fetch))
        .nest_service("/static", ServeDir::new("static"))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("FluxFeed server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
