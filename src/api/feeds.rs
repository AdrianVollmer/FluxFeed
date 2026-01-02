use crate::domain::feed_service;
use crate::web::templates::{FeedFormTemplate, FeedRowTemplate, FeedsListTemplate};
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

pub async fn delete_feed(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    feed_service::delete_feed(&state.db_pool, feed_id).await?;
    Ok(StatusCode::OK)
}

// Error handling
pub enum AppError {
    TemplateError(askama::Error),
    ServiceError(feed_service::FeedServiceError),
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

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::TemplateError(err) => {
                tracing::error!("Template error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error",
                )
                    .into_response()
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
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error",
                )
                    .into_response()
            }
        }
    }
}
