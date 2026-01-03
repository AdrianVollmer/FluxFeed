use crate::api::feeds::AppState;
use crate::domain::{article_service, feed_service};
use crate::infrastructure::repository;
use crate::web::templates::{ArticleRowTemplate, ArticlesListTemplate, ArticleWithFeed};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ArticleListParams {
    pub feed_id: Option<i64>,
    pub is_read: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct MarkAllReadParams {
    pub feed_id: Option<i64>,
}

pub async fn list_articles(
    State(state): State<AppState>,
    Query(params): Query<ArticleListParams>,
) -> Result<Html<String>, AppError> {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    // Get articles
    let articles = article_service::list_articles(
        &state.db_pool,
        params.feed_id,
        params.is_read,
        limit + 1, // Fetch one extra to check if there are more
        offset,
    )
    .await?;

    let has_more = articles.len() > limit as usize;
    let articles_to_show: Vec<_> = articles.into_iter().take(limit as usize).collect();

    // Get feed info for each article
    let mut articles_with_feed = Vec::new();
    for article in articles_to_show {
        let feed = repository::get_feed_by_id(&state.db_pool, article.feed_id)
            .await?
            .unwrap_or_else(|| {
                // Fallback if feed was deleted
                crate::domain::models::Feed {
                    id: article.feed_id,
                    url: String::new(),
                    title: "Unknown Feed".to_string(),
                    description: None,
                    site_url: None,
                    last_fetched_at: None,
                    last_modified: None,
                    etag: None,
                    fetch_interval_minutes: 30,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }
            });

        articles_with_feed.push(ArticleWithFeed {
            article,
            feed_title: feed.title,
        });
    }

    // Get all feeds for the filter
    let feeds = feed_service::list_all_feeds(&state.db_pool).await?;

    // Get unread count
    let unread_count = article_service::get_unread_count(&state.db_pool).await?;

    let template = ArticlesListTemplate {
        articles: articles_with_feed,
        feeds,
        offset,
        limit,
        has_more,
        filter_feed: params.feed_id,
        filter_read: params.is_read,
        unread_count,
    };

    Ok(Html(template.render()?))
}

pub async fn toggle_read_status(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let article = article_service::toggle_read_status(&state.db_pool, article_id).await?;

    // Get feed title
    let feed = repository::get_feed_by_id(&state.db_pool, article.feed_id)
        .await?
        .unwrap_or_else(|| {
            crate::domain::models::Feed {
                id: article.feed_id,
                url: String::new(),
                title: "Unknown Feed".to_string(),
                description: None,
                site_url: None,
                last_fetched_at: None,
                last_modified: None,
                etag: None,
                fetch_interval_minutes: 30,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }
        });

    let template = ArticleRowTemplate {
        article,
        feed_title: feed.title,
    };

    Ok(Html(template.render()?))
}

pub async fn mark_all_read(
    State(state): State<AppState>,
    Query(params): Query<MarkAllReadParams>,
) -> Result<Response, AppError> {
    let count = article_service::mark_all_read(&state.db_pool, params.feed_id).await?;

    tracing::info!("Marked {} articles as read", count);

    // Return HX-Refresh header to reload the page
    Ok((
        StatusCode::OK,
        [("HX-Refresh", "true")],
        format!("Marked {} articles as read", count),
    )
        .into_response())
}

// Error handling
pub enum AppError {
    TemplateError(askama::Error),
    ServiceError(article_service::ArticleServiceError),
    FeedServiceError(feed_service::FeedServiceError),
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

impl From<feed_service::FeedServiceError> for AppError {
    fn from(err: feed_service::FeedServiceError) -> Self {
        AppError::FeedServiceError(err)
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
            AppError::ServiceError(article_service::ArticleServiceError::NotFound) => {
                (StatusCode::NOT_FOUND, "Article not found").into_response()
            }
            AppError::ServiceError(article_service::ArticleServiceError::DatabaseError(err)) => {
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            AppError::FeedServiceError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Feed service error").into_response()
            }
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}
