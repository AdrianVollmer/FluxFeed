use crate::api::feeds::AppState;
use crate::domain::{article_service, feed_service, group_service};
use crate::infrastructure::repository;
use crate::web::templates::{
    ArticleCompactRowTemplate, ArticleCompactRowsTemplate, ArticleRowTemplate, ArticleRowsTemplate,
    ArticleSearchTemplate, ArticleWithFeed, ArticlesListTemplate, ErrorTemplate,
    LoadMoreButtonTemplate,
};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct ArticleListParams {
    pub feed_ids: Option<String>,  // Comma-separated feed IDs
    pub group_ids: Option<String>, // Comma-separated group IDs
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub view: Option<String>,
    pub q: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Deserialize)]
pub struct MarkAllReadParams {
    pub feed_ids: Option<String>, // Comma-separated feed IDs
}

/// Parse comma-separated IDs from query parameter
fn parse_ids(ids_str: Option<&str>) -> Vec<i64> {
    ids_str
        .map(|s| {
            s.split(',')
                .filter_map(|id| id.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

pub async fn list_articles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ArticleListParams>,
) -> Result<Html<String>, AppError> {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    // Parse date parameters
    let date_from = parse_date_param(params.date_from.as_deref(), true);
    let date_to = parse_date_param(params.date_to.as_deref(), false);

    // Parse comma-separated IDs
    let selected_feed_ids = parse_ids(params.feed_ids.as_deref());
    let selected_group_ids = parse_ids(params.group_ids.as_deref());

    // Resolve groups to feed IDs
    let feed_ids = if selected_feed_ids.is_empty() && selected_group_ids.is_empty() {
        None // No filter = all feeds
    } else {
        let ids = group_service::resolve_selection_to_feed_ids(
            &state.db_pool,
            &selected_group_ids,
            &selected_feed_ids,
        )
        .await?;
        if ids.is_empty() {
            None
        } else {
            Some(ids)
        }
    };

    // Get articles with feed data in a single JOIN query (no N+1 problem)
    let articles_with_feed = repository::list_articles_with_feeds(
        &state.db_pool,
        feed_ids,
        params.is_read,
        params.is_starred,
        params.q.clone(),
        date_from,
        date_to,
        limit + 1, // Fetch one extra to check if there are more
        offset,
    )
    .await?;

    let has_more = articles_with_feed.len() > limit as usize;
    let articles_to_show: Vec<_> = articles_with_feed
        .into_iter()
        .take(limit as usize)
        .collect();

    // Check if this is an HTMX pagination request
    let is_htmx = headers.get("HX-Request").is_some();

    // If HTMX request with offset > 0, return just the article rows for pagination
    if is_htmx && offset > 0 {
        return render_htmx_pagination(articles_to_show, has_more, offset, limit, &params);
    }

    // Render full page
    render_full_articles_page(&state, articles_to_show, has_more, offset, limit, &params).await
}

/// Parse date parameter to DateTime (start of day or end of day)
fn parse_date_param(
    date_str: Option<&str>,
    start_of_day: bool,
) -> Option<chrono::DateTime<chrono::Utc>> {
    date_str
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .and_then(|d| {
            if start_of_day {
                d.and_hms_opt(0, 0, 0)
            } else {
                d.and_hms_opt(23, 59, 59)
            }
        })
        .map(|dt| dt.and_utc())
}

/// Render HTMX pagination response (article rows + load more button)
fn render_htmx_pagination(
    articles: Vec<ArticleWithFeed>,
    has_more: bool,
    offset: i64,
    limit: i64,
    params: &ArticleListParams,
) -> Result<Html<String>, AppError> {
    let mut html = String::new();

    // Render article rows using the appropriate template based on view mode
    let view_mode = params.view.as_deref().unwrap_or("cards");
    if view_mode == "compact" {
        let rows_template = ArticleCompactRowsTemplate {
            articles: articles.clone(),
        };
        html.push_str(&rows_template.render()?);
    } else {
        let rows_template = ArticleRowsTemplate {
            articles: articles.clone(),
        };
        html.push_str(&rows_template.render()?);
    }

    // Update the Load More button using out-of-band swap
    if has_more {
        let button_template = LoadMoreButtonTemplate {
            next_offset: offset + limit,
            filter_feed_ids: params.feed_ids.clone(),
            filter_group_ids: params.group_ids.clone(),
            filter_read: params.is_read,
            filter_starred: params.is_starred,
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

    Ok(Html(html))
}

/// Render full articles page with filters and feed list
async fn render_full_articles_page(
    state: &AppState,
    articles: Vec<ArticleWithFeed>,
    has_more: bool,
    offset: i64,
    limit: i64,
    params: &ArticleListParams,
) -> Result<Html<String>, AppError> {
    // Get all feeds and groups for the filter modal
    let feeds = feed_service::list_all_feeds(&state.db_pool).await?;
    let groups = repository::list_groups(&state.db_pool).await?;

    // Build group tree for the filter modal
    let (group_tree, ungrouped_feeds) = group_service::build_group_tree(groups, feeds.clone());

    // Get unread count
    let unread_count = article_service::get_unread_count(&state.db_pool).await?;

    // Parse selected IDs for highlighting in the UI
    let filter_feed_ids = parse_ids(params.feed_ids.as_deref());
    let filter_group_ids = parse_ids(params.group_ids.as_deref());

    let template = ArticlesListTemplate {
        articles,
        feeds,
        group_tree,
        ungrouped_feeds,
        offset,
        limit,
        has_more,
        filter_feed_ids,
        filter_group_ids,
        filter_read: params.is_read,
        filter_starred: params.is_starred,
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
    article_service::toggle_read_status(&state.db_pool, article_id).await?;

    // Get article with feed info in a single JOIN query
    let article_with_feed = repository::get_article_with_feed_by_id(&state.db_pool, article_id)
        .await?
        .ok_or(article_service::ArticleServiceError::NotFound)?;

    let template = ArticleRowTemplate {
        item: article_with_feed,
    };

    Ok(Html(template.render()?))
}

pub async fn toggle_read_status_compact(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    article_service::toggle_read_status(&state.db_pool, article_id).await?;

    // Get article with feed info in a single JOIN query
    let article_with_feed = repository::get_article_with_feed_by_id(&state.db_pool, article_id)
        .await?
        .ok_or(article_service::ArticleServiceError::NotFound)?;

    let template = ArticleCompactRowTemplate {
        item: article_with_feed,
    };

    Ok(Html(template.render()?))
}

pub async fn toggle_starred_status(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    article_service::toggle_starred_status(&state.db_pool, article_id).await?;

    // Get article with feed info in a single JOIN query
    let article_with_feed = repository::get_article_with_feed_by_id(&state.db_pool, article_id)
        .await?
        .ok_or(article_service::ArticleServiceError::NotFound)?;

    let template = ArticleRowTemplate {
        item: article_with_feed,
    };

    Ok(Html(template.render()?))
}

pub async fn toggle_starred_status_compact(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    article_service::toggle_starred_status(&state.db_pool, article_id).await?;

    // Get article with feed info in a single JOIN query
    let article_with_feed = repository::get_article_with_feed_by_id(&state.db_pool, article_id)
        .await?
        .ok_or(article_service::ArticleServiceError::NotFound)?;

    let template = ArticleCompactRowTemplate {
        item: article_with_feed,
    };

    Ok(Html(template.render()?))
}

pub async fn mark_all_read(
    State(state): State<AppState>,
    Query(params): Query<MarkAllReadParams>,
) -> Result<Response, AppError> {
    // Parse feed_ids and mark all as read
    // For simplicity, we mark each feed individually if multiple are specified
    let feed_ids = parse_ids(params.feed_ids.as_deref());

    let count = if feed_ids.is_empty() {
        // Mark all articles as read
        article_service::mark_all_read(&state.db_pool, None).await?
    } else if feed_ids.len() == 1 {
        // Single feed
        article_service::mark_all_read(&state.db_pool, Some(feed_ids[0])).await?
    } else {
        // Multiple feeds - mark each one
        let mut total = 0u64;
        for feed_id in feed_ids {
            total += article_service::mark_all_read(&state.db_pool, Some(feed_id)).await?;
        }
        total
    };

    tracing::info!("Marked {} articles as read", count);

    // Return HX-Refresh header to reload the page
    Ok((
        StatusCode::OK,
        [("HX-Refresh", "true")],
        format!("Marked {} articles as read", count),
    )
        .into_response())
}

pub async fn mark_read_status(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    article_service::mark_as_read(&state.db_pool, article_id).await?;

    // Get article with feed info in a single JOIN query
    let article_with_feed = repository::get_article_with_feed_by_id(&state.db_pool, article_id)
        .await?
        .ok_or(article_service::ArticleServiceError::NotFound)?;

    let template = ArticleRowTemplate {
        item: article_with_feed,
    };

    Ok(Html(template.render()?))
}

pub async fn mark_read_status_compact(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    article_service::mark_as_read(&state.db_pool, article_id).await?;

    // Get article with feed info in a single JOIN query
    let article_with_feed = repository::get_article_with_feed_by_id(&state.db_pool, article_id)
        .await?
        .ok_or(article_service::ArticleServiceError::NotFound)?;

    let template = ArticleCompactRowTemplate {
        item: article_with_feed,
    };

    Ok(Html(template.render()?))
}

// Error handling
#[allow(clippy::enum_variant_names, dead_code)]
pub enum AppError {
    TemplateError(askama::Error),
    ServiceError(article_service::ArticleServiceError),
    FeedServiceError(feed_service::FeedServiceError),
    DatabaseError(sqlx::Error),
    NotFound(String),
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
            AppError::ServiceError(article_service::ArticleServiceError::NotFound) => (
                StatusCode::NOT_FOUND,
                "Not Found".to_string(),
                "The article you're looking for doesn't exist.".to_string(),
            ),
            AppError::ServiceError(article_service::ArticleServiceError::DatabaseError(err)) => {
                tracing::error!("Database error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "A database error occurred. Please try again later.".to_string(),
                )
            }
            AppError::FeedServiceError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
                "An error occurred with the feed service. Please try again later.".to_string(),
            ),
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "A database error occurred. Please try again later.".to_string(),
                )
            }
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                "Not Found".to_string(),
                msg,
            ),
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
    let (articles_with_feed, has_more) =
        if params.q.is_some() || date_from.is_some() || date_to.is_some() {
            // Get articles with feed data in a single JOIN query (no N+1 problem)
            let articles_with_feed = repository::list_articles_with_feeds(
                &state.db_pool,
                None, // No feed filter on search page
                None, // No read filter on search page
                None, // No starred filter on search page
                params.q.clone(),
                date_from,
                date_to,
                limit + 1,
                offset,
            )
            .await?;

            let has_more_results = articles_with_feed.len() > limit as usize;
            let articles_to_show: Vec<_> = articles_with_feed
                .into_iter()
                .take(limit as usize)
                .collect();

            // Check if this is an HTMX pagination request
            let is_htmx = headers.get("HX-Request").is_some();

            if is_htmx && offset > 0 {
                // Return just the article rows for pagination
                let rows_template = ArticleRowsTemplate {
                    articles: articles_to_show,
                };
                return Ok(Html(rows_template.render()?));
            }

            (articles_to_show, has_more_results)
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
