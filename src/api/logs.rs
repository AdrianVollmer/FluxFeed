use crate::api::feeds::AppState;
use crate::infrastructure::repository;
use crate::web::templates::{LoadMoreButtonLogsTemplate, LogRowsTemplate, LogsListTemplate};
use askama::Template;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LogListParams {
    pub feed_id: Option<i64>,
    pub log_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<LogListParams>,
) -> Result<Html<String>, AppError> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    // Get logs with feed info
    let logs = repository::list_logs_with_feeds(
        &state.db_pool,
        params.feed_id,
        params.log_type.as_deref(),
        limit + 1, // Fetch one extra to check if there are more
        offset,
    )
    .await?;

    let has_more = logs.len() > limit as usize;
    let logs_to_show: Vec<_> = logs.into_iter().take(limit as usize).collect();

    // Check if this is an HTMX pagination request
    let is_htmx = headers.get("HX-Request").is_some();

    // If HTMX request with offset > 0, return just the log rows for pagination
    if is_htmx && offset > 0 {
        let mut html = String::new();

        // Render log rows using template
        let rows_template = LogRowsTemplate {
            logs: logs_to_show,
        };
        html.push_str(&rows_template.render()?);

        // Update the Load More button using out-of-band swap
        if has_more {
            let button_template = LoadMoreButtonLogsTemplate {
                next_offset: offset + limit,
                filter_feed: params.feed_id,
                filter_log_type: params.log_type.clone(),
            };
            html.push_str(r#"<div id="load-more-container" hx-swap-oob="true" class="mt-8 text-center">"#);
            html.push_str(&button_template.render()?);
            html.push_str("</div>");
        } else {
            // Remove the Load More button if no more logs
            html.push_str(r#"<div id="load-more-container" hx-swap-oob="true"></div>"#);
        }

        return Ok(Html(html));
    }

    // Get all feeds for the filter dropdown
    let feeds = repository::list_feeds(&state.db_pool).await?;

    let template = LogsListTemplate {
        logs: logs_to_show,
        feeds,
        offset,
        limit,
        has_more,
        filter_feed: params.feed_id,
        filter_log_type: params.log_type,
    };

    Ok(Html(template.render()?))
}

// Error handling
pub enum AppError {
    TemplateError(askama::Error),
    DatabaseError(sqlx::Error),
}

impl From<askama::Error> for AppError {
    fn from(err: askama::Error) -> Self {
        AppError::TemplateError(err)
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
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}
