use axum::{http::StatusCode, Router};
use axum_test::TestServer;
use fluxfeed::api::{articles, feeds, health};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

async fn setup_test_app() -> (TestServer, SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let state = feeds::AppState {
        db_pool: pool.clone(),
    };

    let app = Router::new()
        .route("/health", axum::routing::get(health::check))
        .route("/feeds", axum::routing::get(feeds::list_feeds))
        .route("/feeds", axum::routing::post(feeds::create_feed))
        .route("/feeds/new", axum::routing::get(feeds::show_feed_form))
        .route(
            "/feeds/:id",
            axum::routing::get(feeds::show_feed)
                .post(feeds::update_feed)
                .delete(feeds::delete_feed),
        )
        .route("/articles", axum::routing::get(articles::list_articles))
        .route(
            "/articles/:id/toggle-read",
            axum::routing::post(articles::toggle_read_status),
        )
        .route(
            "/articles/mark-all-read",
            axum::routing::post(articles::mark_all_read),
        )
        .with_state(state);

    let server = TestServer::new(app).unwrap();
    (server, pool)
}

#[tokio::test]
async fn test_health_check() {
    let (server, _pool) = setup_test_app().await;

    let response = server.get("/health").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("\"status\":\"ok\""));
}

#[tokio::test]
async fn test_list_feeds_empty() {
    let (server, _pool) = setup_test_app().await;

    let response = server.get("/feeds").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("FluxFeed"));
}

#[tokio::test]
async fn test_create_feed_invalid_url() {
    let (server, _pool) = setup_test_app().await;

    let response = server
        .post("/feeds")
        .form(&[("url", "not-a-url"), ("title", "Test")])
        .await;

    // Should return an error response
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_show_feed_form() {
    let (server, _pool) = setup_test_app().await;

    let response = server.get("/feeds/new").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("Add Feed") || body.contains("feed"));
}

#[tokio::test]
async fn test_delete_nonexistent_feed() {
    let (server, _pool) = setup_test_app().await;

    let response = server.delete("/feeds/9999").await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_articles_empty() {
    let (server, _pool) = setup_test_app().await;

    let response = server.get("/articles").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    // Should render articles list page
    assert!(body.contains("Articles") || body.contains("article"));
}

#[tokio::test]
async fn test_toggle_read_nonexistent_article() {
    let (server, _pool) = setup_test_app().await;

    let response = server.post("/articles/9999/toggle-read").await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_mark_all_read_with_no_articles() {
    let (server, _pool) = setup_test_app().await;

    let response = server.post("/articles/mark-all-read").await;

    // Should succeed even with no articles
    assert!(
        response.status_code() == StatusCode::OK
            || response.status_code() == StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn test_update_feed_form_deserialization() {
    let (server, pool) = setup_test_app().await;

    // First create a feed directly in the database
    sqlx::query!(
        r#"INSERT INTO feeds (url, title, color, fetch_frequency, fetch_interval_minutes, created_at)
           VALUES ('https://example.com/feed.xml', 'Test Feed', '#3b82f6', 'adaptive', 60, datetime('now'))"#
    )
    .execute(&pool)
    .await
    .expect("Failed to insert test feed");

    // Now try to update it with the same form data that was failing
    let response = server
        .post("/feeds/1")
        .form(&[
            ("title", "Updated Feed"),
            ("url", "https://example.com/feed.xml"),
            ("description", "A test description"),
            ("color", "#3b82f6"),
            ("fetch_frequency", "adaptive"),
            ("custom_hours", "24"),
            ("tag_ids", "1"),
        ])
        .await;

    println!("Response status: {:?}", response.status_code());
    println!("Response body: {}", response.text());

    // The request should be accepted (not 422)
    assert_ne!(
        response.status_code(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Form deserialization failed with 422"
    );
}
