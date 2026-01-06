# N+1 Database Query Problem in Article Handlers

## Severity: CRITICAL

## Category: Performance & Security (DoS Risk)

## Location
- `src/api/articles.rs` (multiple functions)

## Description

The article handlers contain a critical N+1 query problem where feed information is fetched individually for each article in a loop. This occurs in 8 different handler functions:

1. `list_articles` (lines 74-103)
2. `toggle_read_status` (lines 181-198)
3. `toggle_read_status_compact` (lines 218-235)
4. `toggle_starred_status` (lines 256-272)
5. `toggle_starred_status_compact` (lines 294-309)
6. `mark_read_status` (lines 346-363)
7. `mark_read_status_compact` (lines 385-400)
8. `search_articles` (lines 544-562)

## Example Problem Code

In `list_articles` (lines 74-103):

```rust
for article in articles_to_show {
    let feed = repository::get_feed_by_id(&state.db_pool, article.feed_id)
        .await?
        .unwrap_or_else(|| /* fallback Feed */);

    articles_with_feed.push(ArticleWithFeed {
        article,
        feed_title: feed.title.clone(),
        feed_color: feed.color,
    });
}
```

If there are 20 articles from 10 different feeds, this will execute 20 separate SQL queries to fetch feed information.

## Impact

- **Performance degradation**: Each page load with 20 articles could trigger 20+ database queries
- **DoS vulnerability**: Attackers could request large page sizes to amplify database load
- **Scalability issue**: System performance degrades linearly with article count
- **Resource exhaustion**: Database connection pool could be depleted under load

## Solution

1. Create a repository function that joins articles with feeds in a single query:
   ```rust
   pub async fn list_articles_with_feeds(
       pool: &SqlitePool,
       // ... filters ...
   ) -> Result<Vec<ArticleWithFeed>, SqlxError>
   ```

2. Use SQL JOIN to fetch article and feed data together:
   ```sql
   SELECT a.*, f.title as feed_title, f.color as feed_color
   FROM articles a
   INNER JOIN feeds f ON f.id = a.feed_id
   WHERE ...
   ```

3. Alternative: Fetch all unique feed IDs first, then bulk-fetch all feeds in a single query using `WHERE id IN (...)`, then map in memory.

## Testing

Add performance tests to verify query count doesn't scale with article count.
