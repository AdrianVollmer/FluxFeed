mod api;
mod config;
mod infrastructure;
mod web;

use axum::{response::Html, routing::get, Router};
use config::Config;
use infrastructure::database::setup_database;
use std::net::SocketAddr;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, trace::TraceLayer,
};
use web::templates::IndexTemplate;
use askama::Template;

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

    // Build router
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(api::health::check))
        .nest_service("/static", ServeDir::new("static"))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("FluxFeed server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
