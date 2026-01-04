use crate::api::feeds::AppState;
use crate::domain::{article_service, feed_service};
use crate::infrastructure::repository;
use crate::web::templates::{
    ArticleCompactRowTemplate, ArticleCompactRowsTemplate, ArticleRowTemplate, ArticleRowsTemplate,
    ArticleSearchTemplate, ArticleWithFeed, ArticlesListTemplate, LoadMoreButtonTemplate,
};
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
    pub view: Option<String>,
    pub q: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
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

    // Parse date parameters
    let date_from = params
        .date_from
        .as_ref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());

    let date_to = params
        .date_to
        .as_ref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc());

    // Get articles
    let articles = article_service::list_articles(
        &state.db_pool,
        params.feed_id,
        params.is_read,
        params.q.clone(),
        date_from,
        date_to,
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
                    ttl_minutes: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }
            });

        articles_with_feed.push(ArticleWithFeed {
            article,
            feed_title: feed.title.clone(),
            feed_color: feed.color,
        });
    }

    // Check if this is an HTMX pagination request
    let is_htmx = headers.get("HX-Request").is_some();

    // If HTMX request with offset > 0, return just the article rows for pagination
    if is_htmx && offset > 0 {
        let mut html = String::new();

        // Render article rows using the appropriate template based on view mode
        let view_mode = params.view.as_deref().unwrap_or("cards");
        if view_mode == "compact" {
            let rows_template = ArticleCompactRowsTemplate {
                articles: articles_with_feed,
            };
            html.push_str(&rows_template.render()?);
        } else {
            let rows_template = ArticleRowsTemplate {
                articles: articles_with_feed,
            };
            html.push_str(&rows_template.render()?);
        }

        // Update the Load More button using out-of-band swap
        if has_more {
            let button_template = LoadMoreButtonTemplate {
                next_offset: offset + limit,
                filter_feed: params.feed_id,
                filter_read: params.is_read,
                search_query: params.q.clone(),
                date_from: params.date_from.clone(),
                date_to: params.date_to.clone(),
            };
            html.push_str(
                r#"<div id="load-more-container" hx-swap-oob="true" class="mt-8 text-center">"#,
            );
            html.push_str(&button_template.render()?);
            html.push_str("</div>");
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
        search_query: params.q.clone(),
        date_from: params.date_from.clone(),
        date_to: params.date_to.clone(),
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
        .unwrap_or_else(|| crate::domain::models::Feed {
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
            ttl_minutes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
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
        .unwrap_or_else(|| crate::domain::models::Feed {
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
            ttl_minutes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
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

pub async fn search_articles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ArticleListParams>,
) -> Result<Html<String>, AppError> {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    // Parse date parameters
    let date_from = params
        .date_from
        .as_ref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());

    let date_to = params
        .date_to
        .as_ref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc());

    // Only search if we have a query or date filter
    let (articles_with_feed, has_more) = if params.q.is_some() || date_from.is_some() || date_to.is_some() {
        let articles = article_service::list_articles(
            &state.db_pool,
            None,  // No feed filter on search page
            None,  // No read filter on search page
            params.q.clone(),
            date_from,
            date_to,
            limit + 1,
            offset,
        )
        .await?;

        let has_more_results = articles.len() > limit as usize;
        let articles_to_show: Vec<_> = articles.into_iter().take(limit as usize).collect();

        // Get feed info for each article
        let mut articles_with_feed = Vec::new();
        for article in articles_to_show {
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
                        ttl_minutes: None,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    }
                });

            articles_with_feed.push(ArticleWithFeed {
                article,
                feed_title: feed.title.clone(),
                feed_color: feed.color,
            });
        }

        // Check if this is an HTMX pagination request
        let is_htmx = headers.get("HX-Request").is_some();

        if is_htmx && offset > 0 {
            // Return just the article rows for pagination
            let rows_template = ArticleRowsTemplate {
                articles: articles_with_feed,
            };
            return Ok(Html(rows_template.render()?));
        }

        (articles_with_feed, has_more_results)
    } else {
        (Vec::new(), false)
    };

    let template = ArticleSearchTemplate {
        articles: articles_with_feed,
        offset,
        limit,
        has_more,
        search_query: params.q.clone(),
        date_from: params.date_from.clone(),
        date_to: params.date_to.clone(),
    };

    Ok(Html(template.render()?))
}
