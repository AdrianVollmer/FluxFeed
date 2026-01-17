use crate::domain::feed_service;
use crate::infrastructure::{repository, scheduler};
use crate::web::templates::{
    ErrorTemplate, FeedDetailTemplate, FeedFormTemplate, FeedImportFormTemplate,
    FeedImportProgressTemplate, FeedImportResultsTemplate, FeedRowTemplate, FeedsListTemplate,
    ImportResult,
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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Status of an import job
#[derive(Clone, Debug, PartialEq)]
pub enum ImportJobStatus {
    Processing,
    Completed,
}

/// A single feed import result within a job
#[derive(Clone, Debug)]
pub struct ImportJobResult {
    pub url: String,
    pub title: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

/// An import job tracking the progress of a bulk feed import
#[derive(Clone, Debug)]
pub struct ImportJob {
    pub status: ImportJobStatus,
    pub total: usize,
    pub processed: usize,
    pub success_count: usize,
    pub results: Vec<ImportJobResult>,
}

impl ImportJob {
    pub fn new(total: usize) -> Self {
        Self {
            status: ImportJobStatus::Processing,
            total,
            processed: 0,
            success_count: 0,
            results: Vec::with_capacity(total),
        }
    }
}

/// Thread-safe store for import jobs
pub type ImportJobStore = Arc<RwLock<HashMap<String, ImportJob>>>;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub import_jobs: ImportJobStore,
}

#[derive(Deserialize)]
pub struct CreateFeedForm {
    url: String,
    title: Option<String>,
}

/// Deserialize a form field that can be either a single value or multiple values into a Vec
fn deserialize_id_list<'de, D>(deserializer: D) -> Result<Vec<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct IdListVisitor;

    impl<'de> Visitor<'de> for IdListVisitor {
        type Value = Vec<i64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string, integer, or sequence of integers")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value
                .parse::<i64>()
                .map(|id| vec![id])
                .map_err(de::Error::custom)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value])
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value as i64])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut ids = Vec::new();
            while let Some(value) = seq.next_element::<String>()? {
                ids.push(value.parse::<i64>().map_err(de::Error::custom)?);
            }
            Ok(ids)
        }
    }

    deserializer.deserialize_any(IdListVisitor)
}

#[derive(Deserialize)]
pub struct UpdateFeedForm {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub fetch_frequency: String,
    pub color: String,
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub tag_ids: Vec<i64>,
    /// Ignored field sent by the form for custom frequency input
    #[allow(dead_code)]
    #[serde(default)]
    pub custom_hours: Option<String>,
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
) -> Result<impl IntoResponse, AppError> {
    feed_service::delete_feed(&state.db_pool, feed_id).await?;
    Ok((StatusCode::OK, [("HX-Redirect", "/feeds")]))
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

pub async fn show_edit_feed_form(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let feed = repository::get_feed_by_id(&state.db_pool, feed_id)
        .await?
        .ok_or(feed_service::FeedServiceError::NotFound)?;

    let all_tags = repository::list_tags(&state.db_pool).await?;
    let feed_tags = repository::get_feed_tags(&state.db_pool, feed_id).await?;
    let feed_tag_ids: Vec<i64> = feed_tags.iter().map(|t| t.id).collect();

    let template = crate::web::templates::FeedEditFormTemplate {
        feed,
        all_tags,
        feed_tag_ids,
    };
    Ok(Html(template.render()?))
}

pub async fn update_feed(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
    Form(form): Form<UpdateFeedForm>,
) -> Result<impl IntoResponse, AppError> {
    // Validate URL format
    if !form.url.starts_with("http://") && !form.url.starts_with("https://") {
        return Err(AppError::ServiceError(
            feed_service::FeedServiceError::InvalidUrl(
                "URL must start with http:// or https://".to_string(),
            ),
        ));
    }

    // Validate color format
    if !form.color.starts_with('#') || form.color.len() != 7 {
        return Err(AppError::ServiceError(
            feed_service::FeedServiceError::InvalidUrl(
                "Color must be in hex format (#RRGGBB)".to_string(),
            ),
        ));
    }

    // Validate and parse frequency
    let fetch_interval_minutes = feed_service::parse_fetch_frequency(&form.fetch_frequency)?;

    // Convert empty description to None
    let description = form.description.filter(|s| !s.trim().is_empty());

    // Update in database
    repository::update_feed_properties(
        &state.db_pool,
        feed_id,
        &form.title,
        &form.url,
        description.as_deref(),
        &form.fetch_frequency,
        fetch_interval_minutes,
        &form.color,
    )
    .await?;

    // Update feed tags
    repository::set_feed_tags(&state.db_pool, feed_id, &form.tag_ids).await?;

    // Redirect to feed detail page
    Ok((
        StatusCode::OK,
        [("HX-Redirect", format!("/feeds/{}", feed_id))],
    ))
}

pub async fn show_import_form() -> Result<Html<String>, AppError> {
    let template = FeedImportFormTemplate;
    Ok(Html(template.render()?))
}

#[derive(Deserialize)]
pub struct ImportFeedsForm {
    feeds: String,
}

/// Parsed feed entry from the import form
struct ParsedFeedEntry {
    url: String,
    title: Option<String>,
}

/// Parse the import form input into individual feed entries
fn parse_import_input(input: &str) -> Vec<ParsedFeedEntry> {
    input
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            // Split by whitespace - first part is URL, rest is optional title
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            let url = parts[0].to_string();
            let title = parts
                .get(1)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            Some(ParsedFeedEntry { url, title })
        })
        .collect()
}

pub async fn import_feeds(
    State(state): State<AppState>,
    Form(form): Form<ImportFeedsForm>,
) -> Result<Html<String>, AppError> {
    let entries = parse_import_input(&form.feeds);

    if entries.is_empty() {
        // No feeds to import, return empty results
        let template = FeedImportResultsTemplate {
            results: vec![],
            success_count: 0,
        };
        return Ok(Html(template.render()?));
    }

    // Generate a unique job ID
    let job_id = uuid::Uuid::new_v4().to_string();
    let total = entries.len();

    // Create the job
    let job = ImportJob::new(total);

    // Store the job
    {
        let mut jobs = state.import_jobs.write().await;
        jobs.insert(job_id.clone(), job);
    }

    // Spawn background task to process the import
    let job_id_clone = job_id.clone();
    let pool = state.db_pool.clone();
    let import_jobs = state.import_jobs.clone();

    tokio::spawn(async move {
        process_import_job(job_id_clone, entries, pool, import_jobs).await;
    });

    // Return immediately with progress UI that will poll for updates
    let template = FeedImportProgressTemplate {
        job_id,
        total,
        processed: 0,
    };
    Ok(Html(template.render()?))
}

/// Background task to process a feed import job
async fn process_import_job(
    job_id: String,
    entries: Vec<ParsedFeedEntry>,
    pool: SqlitePool,
    import_jobs: ImportJobStore,
) {
    tracing::info!(
        "Starting background import job {} with {} feeds",
        job_id,
        entries.len()
    );

    for entry in entries {
        let result =
            match feed_service::create_feed_deferred(&pool, entry.url.clone(), entry.title.clone())
                .await
            {
                Ok(feed) => ImportJobResult {
                    url: feed.url,
                    title: Some(feed.title),
                    success: true,
                    error: None,
                },
                Err(e) => {
                    let error_msg = match e {
                        feed_service::FeedServiceError::DuplicateUrl => {
                            "Feed URL already exists".to_string()
                        }
                        feed_service::FeedServiceError::InvalidUrl(msg) => msg,
                        feed_service::FeedServiceError::FetchError(msg) => {
                            format!("Failed to fetch feed: {}", msg)
                        }
                        feed_service::FeedServiceError::DatabaseError(err) => {
                            format!("Database error: {}", err)
                        }
                        feed_service::FeedServiceError::SsrfBlocked => {
                            "URL points to internal/private network (blocked for security)"
                                .to_string()
                        }
                        _ => "Unknown error".to_string(),
                    };
                    ImportJobResult {
                        url: entry.url,
                        title: entry.title,
                        success: false,
                        error: Some(error_msg),
                    }
                }
            };

        // Update job state
        {
            let mut jobs = import_jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.processed += 1;
                if result.success {
                    job.success_count += 1;
                }
                job.results.push(result);
            }
        }
    }

    // Mark job as completed
    {
        let mut jobs = import_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = ImportJobStatus::Completed;
        }
    }

    tracing::info!("Completed background import job {}", job_id);
}

/// Get the status of an import job (used for polling)
pub async fn get_import_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Response, AppError> {
    let jobs = state.import_jobs.read().await;

    let Some(job) = jobs.get(&job_id) else {
        return Err(AppError::ServiceError(
            feed_service::FeedServiceError::NotFound,
        ));
    };

    if job.status == ImportJobStatus::Completed {
        // Job is done, return final results
        let results: Vec<ImportResult> = job
            .results
            .iter()
            .map(|r| ImportResult {
                url: r.url.clone(),
                title: r.title.clone(),
                success: r.success,
                error: r.error.clone(),
            })
            .collect();

        let template = FeedImportResultsTemplate {
            results,
            success_count: job.success_count,
        };

        // Clean up the job after returning results (drop the read lock first)
        drop(jobs);
        {
            let mut jobs = state.import_jobs.write().await;
            jobs.remove(&job_id);
        }

        Ok(Html(template.render()?).into_response())
    } else {
        // Job still processing, return progress
        let template = FeedImportProgressTemplate {
            job_id: job_id.clone(),
            total: job.total,
            processed: job.processed,
        };

        Ok(Html(template.render()?).into_response())
    }
}

// Error handling
#[allow(clippy::enum_variant_names, dead_code)]
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
            AppError::ServiceError(feed_service::FeedServiceError::NotFound) => (
                StatusCode::NOT_FOUND,
                "Not Found".to_string(),
                "The feed you're looking for doesn't exist.".to_string(),
            ),
            AppError::ServiceError(feed_service::FeedServiceError::DuplicateUrl) => (
                StatusCode::CONFLICT,
                "Duplicate Feed".to_string(),
                "This feed URL is already in your collection.".to_string(),
            ),
            AppError::ServiceError(feed_service::FeedServiceError::InvalidUrl(msg)) => {
                (StatusCode::BAD_REQUEST, "Invalid URL".to_string(), msg)
            }
            AppError::ServiceError(feed_service::FeedServiceError::DatabaseError(err)) => {
                tracing::error!("Database error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "A database error occurred. Please try again later.".to_string(),
                )
            }
            AppError::ServiceError(feed_service::FeedServiceError::FetchError(msg)) => {
                // Log the actual error for debugging
                tracing::warn!("Feed fetch error: {}", msg);
                (
                    StatusCode::BAD_GATEWAY,
                    "Feed Fetch Failed".to_string(),
                    "Unable to fetch the feed. Please check the URL and try again.".to_string(),
                )
            }
            AppError::ServiceError(feed_service::FeedServiceError::InvalidFrequency) => (
                StatusCode::BAD_REQUEST,
                "Invalid Frequency".to_string(),
                "Fetch frequency must be 'adaptive' or a number of hours between 1-168."
                    .to_string(),
            ),
            AppError::ServiceError(feed_service::FeedServiceError::SsrfBlocked) => (
                StatusCode::BAD_REQUEST,
                "URL Blocked".to_string(),
                "This URL points to an internal or private network address and cannot be used."
                    .to_string(),
            ),
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "A database error occurred. Please try again later.".to_string(),
                )
            }
            AppError::FetchError(msg) => {
                // Log the actual error for debugging
                tracing::warn!("Feed fetch error: {}", msg);
                (
                    StatusCode::BAD_GATEWAY,
                    "Feed Fetch Failed".to_string(),
                    "Unable to fetch the feed. Please check the URL and try again.".to_string(),
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
