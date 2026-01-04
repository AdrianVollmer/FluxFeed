use crate::domain::feed_service;
use crate::infrastructure::{repository, scheduler};
use crate::web::templates::{
    FeedDetailTemplate, FeedFormTemplate, FeedRowTemplate, FeedsListTemplate,
};
use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
}

#[derive(Deserialize)]
pub struct CreateFeedForm {
    url: String,
    title: Option<String>,
}

pub async fn list_feeds(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let feeds = feed_service::list_all_feeds(&state.db_pool).await?;

    let template = FeedsListTemplate { feeds };
    Ok(Html(template.render()?))
}

pub async fn show_feed_form() -> Result<Html<String>, AppError> {
    let template = FeedFormTemplate;
    Ok(Html(template.render()?))
}

pub async fn create_feed(
    State(state): State<AppState>,
    Form(form): Form<CreateFeedForm>,
) -> Result<Html<String>, AppError> {
    let feed = feed_service::create_feed(
        &state.db_pool,
        form.url,
        form.title.filter(|s| !s.is_empty()),
    )
    .await?;

    let template = FeedRowTemplate { feed };
    Ok(Html(template.render()?))
}

pub async fn show_feed(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let feed = repository::get_feed_by_id(&state.db_pool, feed_id)
        .await?
        .ok_or(feed_service::FeedServiceError::NotFound)?;

    // Get tags for this feed
    let tags = repository::get_feed_tags(&state.db_pool, feed_id).await?;

    let template = FeedDetailTemplate { feed, tags };
    Ok(Html(template.render()?))
}

pub async fn delete_feed(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    feed_service::delete_feed(&state.db_pool, feed_id).await?;
    Ok(StatusCode::OK)
}

pub async fn fetch_feed(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let feed = repository::get_feed_by_id(&state.db_pool, feed_id)
        .await?
        .ok_or(feed_service::FeedServiceError::NotFound)?;

    match scheduler::fetch_single_feed(&state.db_pool, &feed).await {
        Ok(scheduler::FetchSingleFeedResult::Updated { new_articles_count }) => {
            tracing::info!(
                "Fetched feed {} with {} new articles",
                feed_id,
                new_articles_count
            );
        }
        Ok(scheduler::FetchSingleFeedResult::NotModified) => {
            tracing::info!("Feed {} not modified", feed_id);
        }
        Err(e) => {
            tracing::warn!("Failed to fetch feed {}: {}", feed_id, e);
            return Err(AppError::FetchError(e.to_string()));
        }
    }

    Ok(StatusCode::OK)
}

// Error handling
pub enum AppError {
    TemplateError(askama::Error),
    ServiceError(feed_service::FeedServiceError),
    DatabaseError(sqlx::Error),
    FetchError(String),
}

impl From<askama::Error> for AppError {
    fn from(err: askama::Error) -> Self {
        AppError::TemplateError(err)
    }
}

impl From<feed_service::FeedServiceError> for AppError {
    fn from(err: feed_service::FeedServiceError) -> Self {
        AppError::ServiceError(err)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::TemplateError(err) => {
                tracing::error!("Template error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            AppError::ServiceError(feed_service::FeedServiceError::NotFound) => {
                (StatusCode::NOT_FOUND, "Feed not found").into_response()
            }
            AppError::ServiceError(feed_service::FeedServiceError::DuplicateUrl) => {
                (StatusCode::CONFLICT, "Feed URL already exists").into_response()
            }
            AppError::ServiceError(feed_service::FeedServiceError::InvalidUrl(msg)) => {
                (StatusCode::BAD_REQUEST, msg).into_response()
            }
            AppError::ServiceError(feed_service::FeedServiceError::DatabaseError(err)) => {
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            AppError::ServiceError(feed_service::FeedServiceError::FetchError(msg)) => (
                StatusCode::BAD_GATEWAY,
                format!("Feed fetch failed: {}", msg),
            )
                .into_response(),
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            AppError::FetchError(msg) => (
                StatusCode::BAD_GATEWAY,
                format!("Feed fetch failed: {}", msg),
            )
                .into_response(),
        }
    }
}
