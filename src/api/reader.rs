use crate::api::feeds::AppState;
use crate::domain::reader_service;
use crate::web::templates::ReaderModeTemplate;
use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

pub async fn show_reader_mode(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let reader_content = reader_service::get_reader_content(&state.db_pool, article_id).await?;

    let template = ReaderModeTemplate {
        article_url: reader_content
            .article
            .url
            .unwrap_or_else(|| String::from("#")),
        title: reader_content.title,
        content: reader_content.content,
        byline: reader_content.byline,
        excerpt: reader_content.excerpt,
    };

    Ok(Html(template.render()?))
}

// Error handling
pub enum AppError {
    TemplateError(askama::Error),
    ReaderServiceError(reader_service::ReaderServiceError),
}

impl From<askama::Error> for AppError {
    fn from(err: askama::Error) -> Self {
        AppError::TemplateError(err)
    }
}

impl From<reader_service::ReaderServiceError> for AppError {
    fn from(err: reader_service::ReaderServiceError) -> Self {
        AppError::ReaderServiceError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::TemplateError(err) => {
                tracing::error!("Template error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            AppError::ReaderServiceError(reader_service::ReaderServiceError::NotFound) => {
                (StatusCode::NOT_FOUND, "Article not found").into_response()
            }
            AppError::ReaderServiceError(reader_service::ReaderServiceError::DatabaseError(
                err,
            )) => {
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            AppError::ReaderServiceError(reader_service::ReaderServiceError::HttpError(err)) => {
                tracing::error!("HTTP error fetching article: {}", err);
                (StatusCode::BAD_GATEWAY, "Failed to fetch article content").into_response()
            }
            AppError::ReaderServiceError(reader_service::ReaderServiceError::ExtractionFailed) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Failed to extract readable content from article",
            )
                .into_response(),
            AppError::ReaderServiceError(reader_service::ReaderServiceError::ReadabilityError(
                err,
            )) => {
                tracing::error!("Readability error: {}", err);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Failed to extract readable content from article",
                )
                    .into_response()
            }
        }
    }
}
