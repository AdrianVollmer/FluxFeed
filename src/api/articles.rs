use crate::api::feeds::AppState;
use crate::domain::{article_service, feed_service};
use crate::infrastructure::repository;
use crate::web::templates::{ArticleCompactRowTemplate, ArticleRowTemplate, ArticlesListTemplate, ArticleWithFeed};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
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
    headers: HeaderMap,
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
                    color: "#3B82F6".to_string(),
                    fetch_frequency: "smart".to_string(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }
            });

        articles_with_feed.push(ArticleWithFeed {
            article,
            feed_title: feed.title,
        });
    }

    // Check if this is an HTMX pagination request
    let is_htmx = headers.get("HX-Request").is_some();

    // If HTMX request with offset > 0, return just the article rows for pagination
    if is_htmx && offset > 0 {
        let mut html = String::new();

        // Render each article row
        for item in &articles_with_feed {
            let row_template = ArticleRowTemplate {
                article: item.article.clone(),
                feed_title: item.feed_title.clone(),
            };
            html.push_str(&row_template.render()?);
            html.push('\n');
        }

        // Update the Load More button using out-of-band swap
        if has_more {
            let next_offset = offset + limit;
            let feed_param = if let Some(feed_id) = params.feed_id {
                format!("&feed_id={}", feed_id)
            } else {
                String::new()
            };
            let read_param = if let Some(is_read) = params.is_read {
                format!("&is_read={}", is_read)
            } else {
                String::new()
            };

            html.push_str(&format!(
                "<div id=\"load-more-container\" hx-swap-oob=\"true\" class=\"mt-8 text-center\">\n\
    <button\n\
        hx-get=\"/articles?offset={}{}{}\"\n\
        hx-target=\"#articles-list\"\n\
        hx-swap=\"beforeend\"\n\
        class=\"btn btn-primary\">\n\
        Load More Articles\n\
    </button>\n\
</div>",
                next_offset, feed_param, read_param
            ));
        } else {
            // Remove the Load More button if no more articles
            html.push_str(r#"<div id="load-more-container" hx-swap-oob="true"></div>"#);
        }

        return Ok(Html(html));
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
                color: "#3B82F6".to_string(),
                fetch_frequency: "smart".to_string(),
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

pub async fn toggle_read_status_compact(
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
                color: "#3B82F6".to_string(),
                fetch_frequency: "smart".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }
        });

    let template = ArticleCompactRowTemplate {
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
