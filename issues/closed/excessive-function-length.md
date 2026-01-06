# Excessive Function Length (Clean Code Violation)

## Severity: MEDIUM

## Category: Readability & Maintainability

## Location
- `src/api/articles.rs`
- `src/infrastructure/scheduler.rs`

## Description

Several functions significantly exceed the project's guideline of 50 lines per function (per CONTRIBUTING.md line 29-30). Long functions are harder to understand, test, and maintain.

## Violations

### Critical Violations (>200 lines)

1. **`scheduler.rs:fetch_single_feed`** - **237 lines** (lines 65-302)
   - Handles feed fetching, error logging, TTL updates, article insertion, and OpenGraph spawning
   - Multiple levels of nested logic
   - Mixes concerns (fetching, parsing, storing, background tasks)

### Major Violations (>100 lines)

2. **`articles.rs:list_articles`** - **173 lines** (lines 35-172)
   - Handles pagination, filtering, HTMX detection, feed fetching, template rendering
   - Contains business logic, data transformation, and presentation logic

3. **`articles.rs:search_articles`** - **97 lines** (lines 502-598)
   - Similar complexity to `list_articles`
   - Duplicates much of the same logic

## Example: `fetch_single_feed` Responsibilities

This single function does:
1. Fetch RSS feed with caching headers
2. Handle HTTP 304 Not Modified
3. Log fetch results (success, not_modified, error)
4. Implement adaptive TTL logic
5. Update feed metadata from RSS
6. Insert articles into database
7. Extract OpenGraph metadata
8. Spawn background tasks for OpenGraph fetching
9. Determine feed-side vs our-side errors
10. Update last_fetched_at based on error type

That's at least 10 distinct responsibilities!

## Impact

- **Cognitive load**: Difficult to understand function flow
- **Testing difficulty**: Hard to write focused unit tests
- **Bug hiding**: Easy to miss edge cases in complex logic
- **Merge conflicts**: More likely in frequently-modified large functions
- **Code review**: Harder to review comprehensively

## Solution

### For `fetch_single_feed`:

Break into focused functions:

```rust
async fn fetch_single_feed(pool: &SqlitePool, feed: &Feed)
    -> Result<FetchSingleFeedResult, Box<dyn std::error::Error>> {

    let fetch_result = fetch_and_parse_feed(feed).await?;

    match fetch_result {
        FetchResult::Updated { feed: parsed_feed, etag, last_modified, ttl } => {
            handle_updated_feed(pool, feed, parsed_feed, etag, last_modified, ttl).await
        }
        FetchResult::NotModified => {
            handle_not_modified_feed(pool, feed).await
        }
    }
}

async fn handle_updated_feed(...) -> Result<FetchSingleFeedResult, ...> {
    log_success(pool, feed).await?;
    update_fetch_interval_if_needed(pool, feed, ttl).await?;
    update_feed_metadata(pool, feed, &parsed_feed, etag, last_modified).await?;
    let new_count = insert_articles(pool, feed, parsed_feed.entries).await?;
    Ok(FetchSingleFeedResult::Updated { new_articles_count: new_count })
}

async fn insert_articles(...) -> Result<usize, ...> {
    let mut count = 0;
    let mut og_queue = Vec::new();

    for entry in entries {
        if let Some(article) = insert_article_from_entry(pool, feed_id, entry).await? {
            count += 1;
            if let Some(url) = article.url {
                og_queue.push((article.id, url));
            }
        }
    }

    spawn_opengraph_fetcher(pool.clone(), og_queue);
    Ok(count)
}
```

### For `list_articles`:

Extract:
- Pagination logic
- Feed data enrichment (once N+1 is fixed)
- HTMX response building
- Template selection

```rust
async fn list_articles(...) -> Result<Html<String>, AppError> {
    let params = parse_article_params(query_params);
    let articles = fetch_articles_with_feeds(pool, &params).await?;
    let pagination = calculate_pagination(&articles, params.limit, params.offset);

    if is_htmx_pagination_request(&headers, params.offset) {
        render_articles_partial(articles, pagination, params).await
    } else {
        render_articles_page(articles, pagination, pool).await
    }
}
```

## Benefits

- Each function has a single, clear purpose
- Easier to test individual behaviors
- Easier to understand control flow
- Follows Single Responsibility Principle
- Reduces nesting levels

## Testing

After refactoring:
1. Existing integration tests should pass
2. Add unit tests for new focused functions
3. Verify no behavioral changes
