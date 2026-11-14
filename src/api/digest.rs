use crate::api::articles::attach_tags_to_articles;
use crate::api::feeds::AppState;
use crate::domain::article_service;
use crate::infrastructure::repository;
use crate::web::templates::{DigestTemplate, ErrorTemplate};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DigestParams {
    pub skip: Option<usize>,
}

#[derive(Deserialize)]
pub struct MarkReadParams {
    pub skip: Option<usize>,
}

/// Marks every article in `feed_id` as read, then redirects back to the same
/// `skip` position - a plain GET link + server redirect rather than an htmx
/// request, so it's a real browser navigation (resets scroll to top) instead of
/// an in-place reload (which preserves scroll).
pub async fn mark_read_and_advance(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
    Query(params): Query<MarkReadParams>,
) -> Result<Redirect, AppError> {
    article_service::mark_all_read(&state.db_pool, Some(feed_id)).await?;
    let skip = params.skip.unwrap_or(0);
    Ok(Redirect::to(&format!("/digest?skip={}", skip)))
}

pub async fn show_digest(
    State(state): State<AppState>,
    Query(params): Query<DigestParams>,
) -> Result<Html<String>, AppError> {
    let skip = params.skip.unwrap_or(0);
    let queue = article_service::get_digest_queue(&state.db_pool).await?;
    let queue_total = queue.len();

    if skip >= queue_total {
        let template = DigestTemplate {
            caught_up: true,
            queue_position: 0,
            queue_total,
            feed: None,
            articles: Vec::new(),
            has_more_articles: false,
            skip: 0,
            has_previous: false,
            prev_skip: 0,
            next_skip: 0,
        };
        return Ok(Html(template.render()?));
    }

    let feed_stats = queue
        .into_iter()
        .nth(skip)
        .expect("skip was already checked against queue_total above");

    let articles_with_feed = repository::list_articles_with_feeds(
        &state.db_pool,
        Some(vec![feed_stats.feed_id]),
        Some(false),
        None,
        None,
        None,
        None,
        21, // fetch one extra to detect "more than 20" (same pattern as the articles page)
        0,
    )
    .await?;

    let has_more_articles = articles_with_feed.len() > 20;
    let articles: Vec<_> = articles_with_feed.into_iter().take(20).collect();
    let articles = attach_tags_to_articles(&state.db_pool, articles).await?;

    let template = DigestTemplate {
        caught_up: false,
        queue_position: skip + 1,
        queue_total,
        feed: Some(feed_stats),
        articles,
        has_more_articles,
        skip,
        has_previous: skip > 0,
        prev_skip: skip.saturating_sub(1),
        next_skip: skip + 1,
    };

    Ok(Html(template.render()?))
}

// Error handling

pub enum AppError {
    TemplateError(askama::Error),
    ServiceError(article_service::ArticleServiceError),
    DatabaseError(sqlx::Error),
}

impl From<askama::Error> for AppError {
    fn from(err: askama::Error) -> Self {
        AppError::TemplateError(err)
    }
}

impl From<article_service::ArticleServiceError> for AppError {
    fn from(err: article_service::ArticleServiceError) -> Self {
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
        let (status_code, status_text, message) = match self {
            AppError::TemplateError(err) => {
                tracing::error!("Template error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "An error occurred while rendering the page. Please try again later."
                        .to_string(),
                )
            }
            AppError::ServiceError(err) => {
                tracing::error!("Digest service error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "An error occurred while loading the digest. Please try again later."
                        .to_string(),
                )
            }
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "A database error occurred. Please try again later.".to_string(),
                )
            }
        };

        let template = ErrorTemplate {
            status_code: status_code.as_u16(),
            status_text,
            message,
        };

        match template.render() {
            Ok(html) => (status_code, Html(html)).into_response(),
            Err(err) => {
                tracing::error!("Error rendering error template: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}
