use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions},
    Error as SqlxError,
};
use std::str::FromStr;

pub async fn setup_database(url: &str) -> Result<SqlitePool, SqlxError> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    // Performance optimizations
    sqlx::query("PRAGMA synchronous = NORMAL") // Faster than FULL, still safe with WAL
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA cache_size = -64000") // 64MB cache
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA temp_store = MEMORY") // Store temp tables in memory
        .execute(&pool)
        .await?;

    Ok(pool)
}
