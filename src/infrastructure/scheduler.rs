use crate::api::feeds::AppState;
use crate::domain::models::NewArticle;
use crate::infrastructure::{repository, rss_fetcher};
use chrono::Utc;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Check if a reqwest error is a connection, DNS, or SSL error (feed-side problems)
fn is_connection_dns_or_ssl_error(err: &reqwest::Error) -> bool {
    // Check for connection errors (connection refused, network unreachable, etc.)
    if err.is_connect() {
        return true;
    }

    // Check for timeout errors (could be feed-side or network issue)
    if err.is_timeout() {
        return true;
    }

    // Check the error message for DNS and SSL-specific errors
    let err_msg = err.to_string().to_lowercase();

    // DNS resolution failures
    if err_msg.contains("dns") || err_msg.contains("name resolution") {
        return true;
    }

    // SSL/TLS errors
    if err_msg.contains("ssl") || err_msg.contains("tls") || err_msg.contains("certificate") {
        return true;
    }

    // Hostname/domain errors
    if err_msg.contains("hostname") || err_msg.contains("domain") {
        return true;
    }

    false
}

pub async fn start_scheduler(state: AppState) -> Result<JobScheduler, Box<dyn std::error::Error>> {
    let scheduler = JobScheduler::new().await?;

    // Fetch all feeds every 5 minutes
    let schedule = "0 */5 * * * *"; // Every 5 minutes

    scheduler
        .add(Job::new_async(schedule, move |_uuid, _lock| {
            let state = state.clone();
            Box::pin(async move {
                if let Err(e) = fetch_all_feeds(&state).await {
                    tracing::error!("Feed fetch cycle failed: {}", e);
                }
            })
        })?)
        .await?;

    scheduler.start().await?;
    tracing::info!("Feed scheduler started (every 5 minutes)");

    Ok(scheduler)
}

/// Fetch and process a single feed, inserting new articles
pub async fn fetch_single_feed(
    pool: &sqlx::SqlitePool,
    feed: &crate::domain::models::Feed,
) -> Result<FetchSingleFeedResult, Box<dyn std::error::Error>> {
    tracing::debug!("Processing feed: {} ({})", feed.title, feed.url);

    let fetcher = rss_fetcher::RssFetcher::new()?;

    match fetcher
        .fetch_feed(
            &feed.url,
            feed.etag.as_deref(),
            feed.last_modified.as_deref(),
        )
        .await
    {
        Ok(rss_fetcher::FetchResult::Updated {
            feed: parsed_feed,
            etag,
            last_modified,
            ttl,
        }) => handle_feed_update(pool, feed, *parsed_feed, etag, last_modified, ttl).await,
        Ok(rss_fetcher::FetchResult::NotModified) => handle_feed_not_modified(pool, feed).await,
        Err(e) => handle_feed_fetch_error(pool, feed, e).await,
    }
}

/// Handle successful feed update: log, update TTL, update metadata, insert articles
async fn handle_feed_update(
    pool: &sqlx::SqlitePool,
    feed: &crate::domain::models::Feed,
    parsed_feed: feed_rs::model::Feed,
    etag: Option<String>,
    last_modified: Option<String>,
    ttl: Option<i64>,
) -> Result<FetchSingleFeedResult, Box<dyn std::error::Error>> {
    tracing::info!(
        "Feed updated: {} ({} entries)",
        feed.title,
        parsed_feed.entries.len()
    );

    // Log successful fetch
    repository::insert_log(pool, feed.id, "success", None, None, None).await?;

    // Store TTL for display purposes (custom mode) or info
    if ttl.is_some() && feed.ttl_minutes != ttl {
        repository::update_feed_ttl_only(pool, feed.id, ttl).await?;
    }

    // Update feed metadata from RSS
    update_feed_metadata_from_rss(pool, feed, &parsed_feed, etag, last_modified).await?;

    // Insert articles and spawn OpenGraph fetching
    let new_articles_count =
        insert_articles_from_entries(pool, feed.id, parsed_feed.entries).await?;

    // Update adaptive fetch interval based on whether we got new articles
    update_adaptive_interval(pool, feed, new_articles_count).await?;

    Ok(FetchSingleFeedResult::Updated { new_articles_count })
}

/// Minimum fetch interval: 1 hour
const MIN_INTERVAL_MINUTES: i64 = 60;
/// Maximum fetch interval: 1 week
const MAX_INTERVAL_MINUTES: i64 = 10080;

/// Update adaptive fetch interval based on whether new articles were found.
///
/// Algorithm:
/// - If no new articles: double the interval
/// - If new articles two fetches in a row: halve the interval
/// - Interval is clamped between 1 hour and 1 week
/// - Only applies to feeds with fetch_frequency = "adaptive"
async fn update_adaptive_interval(
    pool: &sqlx::SqlitePool,
    feed: &crate::domain::models::Feed,
    new_articles_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Only adjust interval for adaptive mode
    if feed.fetch_frequency != "adaptive" {
        return Ok(());
    }

    let has_new_articles = new_articles_count > 0;

    let (new_interval, new_consecutive) = if has_new_articles {
        // We got new articles
        if feed.consecutive_new_articles >= 1 {
            // Two consecutive fetches with new articles: halve the interval
            let halved = (feed.fetch_interval_minutes / 2).max(MIN_INTERVAL_MINUTES);
            tracing::info!(
                "Feed {} had new articles 2x in a row, halving interval: {}m -> {}m",
                feed.id,
                feed.fetch_interval_minutes,
                halved
            );
            (halved, 2) // Cap at 2 to avoid unbounded growth
        } else {
            // First fetch with new articles, just increment counter
            tracing::debug!(
                "Feed {} had new articles, incrementing consecutive counter",
                feed.id
            );
            (feed.fetch_interval_minutes, 1)
        }
    } else {
        // No new articles: double the interval
        let doubled = (feed.fetch_interval_minutes * 2).min(MAX_INTERVAL_MINUTES);
        tracing::info!(
            "Feed {} had no new articles, doubling interval: {}m -> {}m",
            feed.id,
            feed.fetch_interval_minutes,
            doubled
        );
        (doubled, 0) // Reset consecutive counter
    };

    // Only update if something changed
    if new_interval != feed.fetch_interval_minutes
        || new_consecutive != feed.consecutive_new_articles
    {
        repository::update_adaptive_fetch_state(pool, feed.id, new_consecutive, new_interval)
            .await?;
    }

    Ok(())
}

/// Extract and update feed metadata from RSS feed
async fn update_feed_metadata_from_rss(
    pool: &sqlx::SqlitePool,
    feed: &crate::domain::models::Feed,
    parsed_feed: &feed_rs::model::Feed,
    etag: Option<String>,
    last_modified: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rss_title = parsed_feed.title.as_ref().map(|t| t.content.clone());
    let rss_description = parsed_feed.description.as_ref().map(|d| d.content.clone());
    let feed_site_url = parsed_feed.links.first().map(|link| link.href.clone());

    // Implement description fallback logic:
    // If no description exists in DB, use RSS feed's title
    // If RSS feed has no title, use URL as description
    let feed_description = if feed.description.is_none() {
        rss_description
            .or_else(|| rss_title.clone())
            .or_else(|| Some(feed.url.clone()))
    } else {
        // Keep existing description
        None
    };

    repository::update_feed_details(
        pool,
        feed.id,
        rss_title,
        feed_description,
        feed_site_url,
        etag,
        last_modified,
    )
    .await?;

    Ok(())
}

/// Insert articles from feed entries and spawn OpenGraph fetching
async fn insert_articles_from_entries(
    pool: &sqlx::SqlitePool,
    feed_id: i64,
    entries: Vec<feed_rs::model::Entry>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut new_articles_count = 0;
    let mut article_ids_to_fetch = Vec::new();

    for entry in entries {
        let guid = generate_guid(&entry);
        let title = extract_title(&entry);
        let url = extract_url(&entry);
        let content = extract_content(&entry);
        let summary = extract_summary(&entry);
        let author = extract_author(&entry);
        let published_at = extract_published_date(&entry);

        // Insert article without OpenGraph data
        match repository::insert_article_if_new(
            pool,
            NewArticle {
                feed_id,
                guid,
                title,
                url: url.clone(),
                content,
                summary,
                author,
                published_at,
                og_image: None,
                og_description: None,
                og_site_name: None,
            },
        )
        .await
        {
            Ok(Some(article)) => {
                new_articles_count += 1;
                // Queue this article for OpenGraph fetching if it has a URL
                if let Some(article_url) = url {
                    article_ids_to_fetch.push((article.id, article_url));
                }
            }
            Ok(None) => {
                // Article already exists (duplicate)
            }
            Err(e) => {
                tracing::warn!("Failed to insert article: {}", e);
            }
        }
    }

    // Spawn background task to fetch OpenGraph metadata
    if !article_ids_to_fetch.is_empty() {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            fetch_opengraph_for_articles(pool_clone, article_ids_to_fetch).await;
        });
    }

    Ok(new_articles_count)
}

/// Handle feed not modified: log, update last_fetched_at, and adjust adaptive interval
async fn handle_feed_not_modified(
    pool: &sqlx::SqlitePool,
    feed: &crate::domain::models::Feed,
) -> Result<FetchSingleFeedResult, Box<dyn std::error::Error>> {
    tracing::debug!("Feed not modified: {}", feed.title);

    // Log not modified fetch
    repository::insert_log(pool, feed.id, "not_modified", None, None, None).await?;

    // Just update last_fetched_at
    repository::touch_feed(pool, feed.id).await?;

    // Not modified means no new articles - update adaptive interval
    update_adaptive_interval(pool, feed, 0).await?;

    Ok(FetchSingleFeedResult::NotModified)
}

/// Handle feed fetch error: log error, determine retry strategy
async fn handle_feed_fetch_error(
    pool: &sqlx::SqlitePool,
    feed: &crate::domain::models::Feed,
    error: rss_fetcher::FetchError,
) -> Result<FetchSingleFeedResult, Box<dyn std::error::Error>> {
    tracing::warn!("Failed to fetch feed {}: {}", feed.url, error);

    // Extract error details for logging
    let (log_type, status_code, retry_after) = match &error {
        rss_fetcher::FetchError::RequestFailed {
            status,
            retry_after,
            ..
        } => {
            let log_type = if status.as_u16() == 429 {
                "rate_limited"
            } else {
                "error"
            };
            (
                log_type,
                Some(status.as_u16() as i32),
                retry_after.as_deref(),
            )
        }
        _ => ("error", None, None),
    };

    let error_message = error.to_string();

    // Log the fetch failure
    repository::insert_log(
        pool,
        feed.id,
        log_type,
        status_code,
        Some(&error_message),
        retry_after,
    )
    .await?;

    // Determine if this is a "feed-side" or "our-side" problem
    // Feed-side problems: connection refused, DNS errors, SSL errors
    // → Update last_fetched_at to respect normal interval (avoid hammering broken feeds)
    // Our-side problems: HTTP errors, parse errors, other issues
    // → Don't update last_fetched_at so we retry in 5 minutes (next scheduler cycle)
    let is_feed_side_problem = match &error {
        rss_fetcher::FetchError::NetworkError(req_err) => {
            // Check for connection/DNS/SSL errors
            is_connection_dns_or_ssl_error(req_err)
        }
        _ => false, // HTTP errors, parse errors = our-side problem
    };

    if is_feed_side_problem {
        tracing::info!(
            "Feed-side problem for {}, will retry based on normal interval",
            feed.url
        );
        repository::touch_feed(pool, feed.id).await?;
    } else {
        tracing::info!(
            "Transient/our-side problem for {}, will retry in 5 minutes",
            feed.url
        );
        // Don't update last_fetched_at - will be retried on next scheduler cycle
    }

    Err(error.into())
}

pub enum FetchSingleFeedResult {
    Updated { new_articles_count: usize },
    NotModified,
}

async fn fetch_all_feeds(state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting feed fetch cycle");

    // Get all feeds that need updating
    let feeds = repository::get_feeds_to_update(&state.db_pool).await?;

    tracing::info!("Found {} feeds to update", feeds.len());

    if feeds.is_empty() {
        return Ok(());
    }

    let mut new_articles_total = 0;
    let mut updated_feeds_count = 0;

    // Process feeds sequentially with rate limiting
    for feed in feeds {
        match fetch_single_feed(&state.db_pool, &feed).await {
            Ok(FetchSingleFeedResult::Updated { new_articles_count }) => {
                new_articles_total += new_articles_count;
                updated_feeds_count += 1;
            }
            Ok(FetchSingleFeedResult::NotModified) => {
                // Feed not modified
            }
            Err(e) => {
                tracing::warn!("Failed to fetch feed {}: {}", feed.url, e);
            }
        }

        // Rate limiting: 500ms delay between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::info!(
        "Feed fetch cycle complete: {} feeds updated, {} new articles",
        updated_feeds_count,
        new_articles_total
    );

    Ok(())
}

// Helper functions to extract data from feed entries

fn generate_guid(entry: &feed_rs::model::Entry) -> String {
    // Use entry ID if available and not empty
    if !entry.id.is_empty() {
        entry.id.clone()
    } else if let Some(link) = entry.links.first() {
        // Generate from link + title
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.as_str())
            .unwrap_or("");
        format!("{}-{}", link.href, title)
    } else {
        // Fallback: use title + published date
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.as_str())
            .unwrap_or("untitled");
        let date = entry
            .published
            .or(entry.updated)
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        format!("{}-{}", title, date)
    }
}

fn extract_title(entry: &feed_rs::model::Entry) -> String {
    entry
        .title
        .as_ref()
        .map(|t| t.content.clone())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn extract_url(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.links.first().map(|link| link.href.clone())
}

fn extract_content(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.content.as_ref().and_then(|c| {
        c.body.as_ref().map(|body| {
            // Sanitize HTML to prevent XSS attacks
            // Don't truncate - let CSS handle visual limiting to avoid breaking HTML tags
            ammonia::clean(body)
        })
    })
}

fn extract_summary(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.summary.as_ref().map(|s| {
        // Sanitize HTML to prevent XSS attacks
        // Don't truncate - let CSS handle visual limiting to avoid breaking HTML tags
        ammonia::clean(&s.content)
    })
}

fn extract_author(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.authors.first().map(|author| author.name.clone())
}

fn extract_published_date(entry: &feed_rs::model::Entry) -> Option<chrono::DateTime<Utc>> {
    entry
        .published
        .or(entry.updated)
        .map(|dt| dt.with_timezone(&Utc))
}

/// Fetch OpenGraph metadata for multiple articles in the background
async fn fetch_opengraph_for_articles(
    pool: sqlx::SqlitePool,
    articles: Vec<(i64, String)>, // (article_id, url)
) {
    let article_count = articles.len();
    tracing::info!(
        "Starting background OpenGraph fetch for {} articles",
        article_count
    );

    for (article_id, url) in articles {
        // Fetch OpenGraph metadata
        let (og_image, og_description, og_site_name) = extract_opengraph_from_url(&url).await;

        // Update article with OpenGraph data if any was found
        if og_image.is_some() || og_description.is_some() || og_site_name.is_some() {
            match repository::update_article_opengraph(
                &pool,
                article_id,
                og_image,
                og_description,
                og_site_name,
            )
            .await
            {
                Ok(_) => {
                    tracing::debug!("Updated OpenGraph data for article {}", article_id);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to update OpenGraph for article {}: {}",
                        article_id,
                        e
                    );
                }
            }
        }

        // Small delay between requests to be a good citizen
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tracing::info!(
        "Completed background OpenGraph fetch for {} articles",
        article_count
    );
}

async fn extract_opengraph_from_url(
    url_str: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    // Try to fetch and parse OpenGraph metadata
    match webpage::Webpage::from_url(url_str, webpage::WebpageOptions::default()) {
        Ok(webpage) => {
            // Sanitize and validate og_image URL (prevent XSS via malicious URLs)
            let og_image = webpage
                .html
                .opengraph
                .images
                .first()
                .map(|img| img.url.as_str())
                .and_then(|url| {
                    // Only allow http:// and https:// URLs to prevent javascript: or data: URIs
                    if url.starts_with("http://") || url.starts_with("https://") {
                        Some(url.to_string())
                    } else {
                        tracing::warn!("Rejected unsafe og:image URL from {}: {}", url_str, url);
                        None
                    }
                });

            // Sanitize og_description to prevent stored XSS
            let og_description = webpage
                .html
                .opengraph
                .properties
                .get("og:description")
                .map(|s| ammonia::clean(s));

            // Sanitize og_site_name to prevent stored XSS
            let og_site_name = webpage
                .html
                .opengraph
                .properties
                .get("og:site_name")
                .map(|s| ammonia::clean(s));

            tracing::debug!(
                "Extracted OpenGraph from {}: image={:?}, desc={:?}, site={:?}",
                url_str,
                og_image.is_some(),
                og_description.is_some(),
                og_site_name.is_some()
            );

            (og_image, og_description, og_site_name)
        }
        Err(e) => {
            tracing::debug!("Failed to extract OpenGraph from {}: {}", url_str, e);
            (None, None, None)
        }
    }
}
