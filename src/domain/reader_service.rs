use crate::domain::models::Article;
use crate::infrastructure::repository;
use dom_smoothie::Readability;
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReaderServiceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Article not found")]
    NotFound,

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Failed to extract readable content: {0}")]
    ReadabilityError(#[from] dom_smoothie::ReadabilityError),

    #[error("Failed to extract readable content")]
    ExtractionFailed,
}

pub struct ReaderContent {
    pub article: Article,
    pub title: String,
    pub content: String,
    pub byline: Option<String>,
    pub excerpt: Option<String>,
}

pub async fn get_reader_content(
    pool: &SqlitePool,
    article_id: i64,
) -> Result<ReaderContent, ReaderServiceError> {
    // Get the article from database
    let article = repository::get_article_by_id(pool, article_id)
        .await?
        .ok_or(ReaderServiceError::NotFound)?;

    // Get the article URL
    let article_url = article
        .url
        .as_ref()
        .ok_or(ReaderServiceError::ExtractionFailed)?;

    // Fetch the article content from the URL
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(crate::user_agent())
        .build()?;

    let response = client.get(article_url).send().await?;
    let html = response.text().await?;

    // Process with dom_smoothie
    let mut readability = Readability::new(html, Some(article_url.as_str()), None)?;
    let article_content = readability.parse()?;

    Ok(ReaderContent {
        article: article.clone(),
        title: article_content.title,
        content: article_content.content.to_string(),
        byline: article_content.byline,
        excerpt: article_content.excerpt,
    })
}
