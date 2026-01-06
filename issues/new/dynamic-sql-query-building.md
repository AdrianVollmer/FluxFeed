# Dynamic SQL Query Building (Security & Maintainability Risk)

## Severity: MEDIUM

## Category: Security & Maintainability

## Location
- `src/infrastructure/repository.rs:237-308` (`list_articles`)
- `src/infrastructure/repository.rs:468-524` (`list_logs_with_feeds`)

## Description

Two functions build SQL queries dynamically by string concatenation. While they do use parameterized queries (preventing SQL injection), this approach is:

1. **Error-prone**: Easy to introduce SQL injection if not careful
2. **Fragile**: Hard to maintain as query complexity grows
3. **Type-unsafe**: No compile-time verification of SQL correctness
4. **Harder to review**: More mental overhead to verify security

## Example: `list_articles` (lines 248-308)

```rust
let mut query_str = String::from("SELECT a.* FROM articles a");

if search_query.is_some() {
    query_str.push_str(" INNER JOIN articles_fts ON a.id = articles_fts.rowid");
}

query_str.push_str(" WHERE 1=1");

if search_query.is_some() {
    query_str.push_str(" AND articles_fts MATCH ?");
}

if feed_id.is_some() {
    query_str.push_str(" AND a.feed_id = ?");
}

// ... more conditionals ...

let mut query = sqlx::query_as::<_, Article>(&query_str);

// Bind parameters in correct order
if let Some(search) = search_query {
    query = query.bind(search);
}
if let Some(fid) = feed_id {
    query = query.bind(fid);
}
// ... more binds ...
```

## Problems

1. **Brittle parameter binding**: Must bind parameters in exact same order as conditional query building. Easy to get wrong.

2. **No compile-time verification**: typos like `"AND a.fed_id = ?"` (missing 'e') won't be caught until runtime.

3. **Complex to audit**: Need to trace through conditionals to verify:
   - All query string appends match parameter binds
   - No SQL injection via variable interpolation
   - Correct number of `?` placeholders

4. **`list_logs_with_feeds` is worse**: Uses a `Vec<String>` for bindings with string conversion, adding more fragility.

## Example of Potential Bug

If someone adds a filter but forgets to bind:

```rust
// Query string modified
if some_filter.is_some() {
    query_str.push_str(" AND a.some_field = ?");  // Added
}

// But binding is forgotten - query will fail or bind wrong parameter!
```

## Better Alternatives

### Option 1: Use SQLx Query Builder (if available)

SQLx doesn't have a full query builder, but you can use conditional logic more safely:

```rust
let base_query = if search_query.is_some() {
    "SELECT a.* FROM articles a INNER JOIN articles_fts ON a.id = articles_fts.rowid"
} else {
    "SELECT a.* FROM articles a"
};

let mut conditions = Vec::new();
if search_query.is_some() { conditions.push("articles_fts MATCH ?"); }
if feed_id.is_some() { conditions.push("a.feed_id = ?"); }
if is_read.is_some() { conditions.push("a.is_read = ?"); }

let where_clause = if conditions.is_empty() {
    String::new()
} else {
    format!(" WHERE {}", conditions.join(" AND "))
};

let query_str = format!("{}{} ORDER BY ...", base_query, where_clause);
```

Still not ideal, but clearer structure.

### Option 2: Create Specific Query Functions

Instead of one über-flexible function, create targeted functions:

```rust
async fn list_articles_by_feed(pool: &SqlitePool, feed_id: i64, limit: i64, offset: i64) -> ...
async fn search_articles_fts(pool: &SqlitePool, query: &str, limit: i64, offset: i64) -> ...
async fn list_unread_articles(pool: &SqlitePool, limit: i64, offset: i64) -> ...
```

Each function has a simple, static SQL query that's type-checked.

### Option 3: Use SQLx Compile-Time Verification

For complex queries, use `sqlx::query!` macro:

```rust
let articles = if let Some(fid) = feed_id {
    sqlx::query_as!(
        Article,
        r#"SELECT * FROM articles WHERE feed_id = ? ORDER BY published_at DESC LIMIT ? OFFSET ?"#,
        fid, limit, offset
    ).fetch_all(pool).await?
} else {
    sqlx::query_as!(
        Article,
        r#"SELECT * FROM articles ORDER BY published_at DESC LIMIT ? OFFSET ?"#,
        limit, offset
    ).fetch_all(pool).await?
};
```

More verbose but type-safe and SQL-injection proof.

### Option 4: Use a Query Builder Library

Consider using a library like `sea-query` for compile-time safe dynamic queries:

```rust
let mut query = Query::select()
    .from(Articles::Table)
    .columns([Articles::Id, Articles::Title, /* ... */])
    .to_owned();

if let Some(fid) = feed_id {
    query.and_where(Expr::col(Articles::FeedId).eq(fid));
}

if let Some(read) = is_read {
    query.and_where(Expr::col(Articles::IsRead).eq(read));
}

let (sql, values) = query.build_sqlx(SqliteQueryBuilder);
```

## Recommendation

1. **Short-term**: Extract the query building logic into a dedicated, well-tested helper function with clear documentation
2. **Medium-term**: Split the uber-flexible function into specific query functions for common use cases
3. **Long-term**: Consider migrating to a query builder library or using static queries with type-safe composition

## Testing

1. Add unit tests for query building logic covering all filter combinations
2. Add integration tests that verify query results match expectations
3. Add property-based tests to ensure parameter binding order is always correct
