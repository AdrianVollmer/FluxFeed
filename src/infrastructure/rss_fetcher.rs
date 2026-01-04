use feed_rs::parser;
use reqwest::{header, Client, StatusCode};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("HTTP request failed with status {status}: {message}")]
    RequestFailed {
        status: StatusCode,
        message: String,
        retry_after: Option<String>,
    },

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Feed parsing failed: {0}")]
    ParseError(String),

    #[error("Invalid feed format")]
    InvalidFormat,
}

pub enum FetchResult {
    Updated {
        feed: feed_rs::model::Feed,
        etag: Option<String>,
        last_modified: Option<String>,
    },
    NotModified,
}

pub struct RssFetcher {
    client: Client,
}

impl RssFetcher {
    pub fn new() -> Result<Self, FetchError> {
        let client = Client::builder()
            .user_agent("FluxFeed/0.1.0 (+https://github.com/fluxfeed/fluxfeed)")
            .gzip(true)
            .brotli(true)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self { client })
    }

    pub async fn fetch_feed(
        &self,
        url: &str,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<FetchResult, FetchError> {
        let mut request = self.client.get(url);

        // Add conditional GET headers
        if let Some(etag) = etag {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        if let Some(modified) = last_modified {
            request = request.header(header::IF_MODIFIED_SINCE, modified);
        }

        tracing::debug!("Fetching feed: {}", url);
        let response = request.send().await?;

        // Handle 304 Not Modified (feed unchanged)
        if response.status() == StatusCode::NOT_MODIFIED {
            tracing::debug!("Feed not modified: {}", url);
            return Ok(FetchResult::NotModified);
        }

        // Check for successful response
        if !response.status().is_success() {
            let status = response.status();
            let retry_after = response
                .headers()
                .get(header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .map(String::from);

            tracing::warn!("Feed fetch failed with status {}: {}", status, url);

            let message = format!(
                "{} - {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            );

            return Err(FetchError::RequestFailed {
                status,
                message,
                retry_after,
            });
        }

        // Extract new cache headers
        let new_etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let new_last_modified = response
            .headers()
            .get(header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        tracing::debug!(
            "Feed response headers - ETag: {:?}, Last-Modified: {:?}",
            new_etag,
            new_last_modified
        );

        let body = response.text().await?;

        // Parse the feed
        let feed = parser::parse(body.as_bytes()).map_err(|e| {
            tracing::error!("Feed parsing error for {}: {}", url, e);
            FetchError::ParseError(e.to_string())
        })?;

        tracing::info!(
            "Successfully parsed feed: {} ({} entries)",
            feed.title.as_ref().map(|t| t.content.as_str()).unwrap_or("Untitled"),
            feed.entries.len()
        );

        Ok(FetchResult::Updated {
            feed,
            etag: new_etag,
            last_modified: new_last_modified,
        })
    }
}

impl Default for RssFetcher {
    fn default() -> Self {
        Self::new().expect("Failed to create RssFetcher")
    }
}
