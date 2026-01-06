use feed_rs::parser;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::{header, Client, StatusCode};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
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
        feed: Box<feed_rs::model::Feed>,
        etag: Option<String>,
        last_modified: Option<String>,
        ttl: Option<i64>,
    },
    NotModified,
}

pub struct RssFetcher {
    client: Client,
}

impl RssFetcher {
    pub fn new() -> Result<Self, FetchError> {
        let client = Client::builder()
            .user_agent(crate::user_agent())
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

        // Extract TTL from raw XML before parsing
        let ttl = extract_ttl_from_xml(&body);

        // Parse the feed
        let feed = parser::parse(body.as_bytes()).map_err(|e| {
            tracing::error!("Feed parsing error for {}: {}", url, e);
            FetchError::ParseError(e.to_string())
        })?;

        tracing::info!(
            "Successfully parsed feed: {} ({} entries)",
            feed.title
                .as_ref()
                .map(|t| t.content.as_str())
                .unwrap_or("Untitled"),
            feed.entries.len()
        );

        Ok(FetchResult::Updated {
            feed: Box::new(feed),
            etag: new_etag,
            last_modified: new_last_modified,
            ttl,
        })
    }
}

impl Default for RssFetcher {
    fn default() -> Self {
        Self::new().expect("Failed to create RssFetcher")
    }
}

/// Extract TTL (Time To Live) from RSS 2.0 feed XML
/// Returns TTL in minutes if found
fn extract_ttl_from_xml(xml: &str) -> Option<i64> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut in_channel = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"channel" => {
                in_channel = true;
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"ttl" && in_channel => {
                // Next text event contains the TTL value
                buf.clear();
                if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                    if let Ok(ttl_str) = t.unescape() {
                        if let Ok(ttl) = ttl_str.parse::<i64>() {
                            if ttl > 0 {
                                tracing::debug!("Extracted TTL: {} minutes", ttl);
                                return Some(ttl);
                            }
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"channel" => {
                in_channel = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::debug!("Error parsing XML for TTL: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    None
}
