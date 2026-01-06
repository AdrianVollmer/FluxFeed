# Massive Code Duplication: Fallback Feed Creation

## Severity: MEDIUM

## Category: Maintainability (DRY Violation)

## Location
- `src/api/articles.rs` (8 instances)

## Description

The same fallback `Feed` struct creation code is duplicated 8 times across different handler functions in `articles.rs`. This violates the DRY (Don't Repeat Yourself) principle and makes maintenance difficult.

## Duplicated Code Instances

1. `list_articles` (lines 78-96)
2. `toggle_read_status` (lines 183-198)
3. `toggle_read_status_compact` (lines 220-235)
4. `toggle_starred_status` (lines 258-272)
5. `toggle_starred_status_compact` (lines 294-309)
6. `mark_read_status` (lines 348-363)
7. `mark_read_status_compact` (lines 385-400)
8. `search_articles` (lines 547-562)

## Example Duplicated Code

```rust
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
```

This 18-line block is copy-pasted 8 times!

## Impact

- **Maintainability**: Changes require updating 8 locations
- **Bug risk**: Easy to miss updates in some locations
- **Code bloat**: ~144 lines of duplicated code
- **Readability**: Obscures actual business logic

## Solution

Extract to a helper function:

```rust
async fn get_feed_or_default(
    pool: &SqlitePool,
    feed_id: i64,
) -> Result<Feed, sqlx::Error> {
    repository::get_feed_by_id(pool, feed_id)
        .await
        .map(|opt| opt.unwrap_or_else(|| Feed::default_with_id(feed_id)))
}
```

Or add to the `Feed` model:

```rust
impl Feed {
    pub fn default_with_id(feed_id: i64) -> Self {
        Self {
            id: feed_id,
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
    }
}
```

Then replace all 8 instances with:
```rust
let feed = get_feed_or_default(&state.db_pool, article.feed_id).await?;
```

## Additional Note

This duplication would be eliminated entirely by fixing the N+1 query problem (see `n-plus-one-query-problem.md`), as feed data would be fetched via JOIN and wouldn't need fallback logic per article.

## Testing

Existing tests should pass after refactoring.
