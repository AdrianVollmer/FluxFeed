#!/usr/bin/env python3
"""
Stress test script for FluxFeed.

Creates a SQLite database in /tmp with:
- 1000+ feeds (with invalid URLs)
- 100k+ articles
- Random tags assigned to feeds
- Lorem ipsum content with counters in titles
"""

import hashlib
import random
import re
import sqlite3
import uuid
from datetime import datetime, timedelta
from pathlib import Path

DB_PATH = Path("/tmp/fluxfeed-stress-test.db")
MIGRATIONS_DIR = Path(__file__).parent.parent / "migrations"

NUM_FEEDS = 1000
NUM_ARTICLES = 100_000
NUM_TAGS = 50

LOREM_WORDS = [
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing",
    "elit", "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore",
    "et", "dolore", "magna", "aliqua", "enim", "ad", "minim", "veniam",
    "quis", "nostrud", "exercitation", "ullamco", "laboris", "nisi",
    "aliquip", "ex", "ea", "commodo", "consequat", "duis", "aute", "irure",
    "in", "reprehenderit", "voluptate", "velit", "esse", "cillum", "fugiat",
    "nulla", "pariatur", "excepteur", "sint", "occaecat", "cupidatat", "non",
    "proident", "sunt", "culpa", "qui", "officia", "deserunt", "mollit",
    "anim", "id", "est", "laborum", "cras", "justo", "odio", "dapibus",
    "facilisis", "egestas", "felis", "donec", "pulvinar", "neque", "laoreet",
    "suspendisse", "interdum", "faucibus", "nisl", "tincidunt", "integer",
    "posuere", "erat", "ante", "venenatis", "morbi", "leo", "risus", "porta",
    "ac", "vestibulum", "at", "eros", "praesent", "blandit", "euismod",
]

TAG_NAMES = [
    "tech", "news", "science", "politics", "sports", "entertainment",
    "gaming", "programming", "rust", "python", "javascript", "linux",
    "open-source", "security", "ai", "machine-learning", "data-science",
    "web-dev", "mobile", "devops", "cloud", "database", "networking",
    "hardware", "software", "tutorials", "reviews", "opinion", "analysis",
    "breaking", "daily", "weekly", "monthly", "featured", "popular",
    "trending", "archived", "important", "bookmarked", "read-later",
    "favorites", "must-read", "recommended", "community", "official",
    "indie", "mainstream", "international", "local",
]

COLORS = [
    "#3B82F6", "#EF4444", "#10B981", "#F59E0B", "#8B5CF6",
    "#EC4899", "#06B6D4", "#84CC16", "#F97316", "#6366F1",
]

STYLES = ["solid", "outline", "subtle"]


def lorem_words(count: int) -> str:
    """Generate random lorem ipsum words."""
    return " ".join(random.choices(LOREM_WORDS, k=count))


def lorem_sentence() -> str:
    """Generate a random lorem ipsum sentence."""
    sentence = lorem_words(random.randint(5, 15))
    return sentence.capitalize() + "."


def lorem_paragraph() -> str:
    """Generate a random lorem ipsum paragraph."""
    sentences = [lorem_sentence() for _ in range(random.randint(3, 8))]
    return " ".join(sentences)


def lorem_paragraphs(count: int) -> str:
    """Generate multiple lorem ipsum paragraphs as HTML."""
    paragraphs = [f"<p>{lorem_paragraph()}</p>" for _ in range(count)]
    return "\n".join(paragraphs)


def run_migrations(conn: sqlite3.Connection) -> None:
    """Run all migrations in order, tracking them in _sqlx_migrations."""
    cursor = conn.cursor()

    # Create the sqlx migrations tracking table
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            success BOOLEAN NOT NULL,
            checksum BLOB NOT NULL,
            execution_time BIGINT NOT NULL
        )
    """)
    conn.commit()

    migration_files = sorted(MIGRATIONS_DIR.glob("*.sql"))
    for migration_file in migration_files:
        print(f"  Running migration: {migration_file.name}")
        sql = migration_file.read_text()

        # Parse version and description from filename
        # Format: 20260102000001_initial_schema.sql
        match = re.match(r"(\d+)_(.+)\.sql", migration_file.name)
        if not match:
            raise ValueError(f"Invalid migration filename: {migration_file.name}")

        version = int(match.group(1))
        description = match.group(2).replace("_", " ")

        # Compute SHA-384 checksum (what sqlx uses)
        checksum = hashlib.sha384(sql.encode()).digest()

        # Run the migration
        conn.executescript(sql)

        # Record the migration
        cursor.execute(
            """
            INSERT INTO _sqlx_migrations
                (version, description, success, checksum, execution_time)
            VALUES (?, ?, 1, ?, 1000000)
            """,
            (version, description, checksum),
        )
        conn.commit()


def create_tags(conn: sqlite3.Connection) -> list[int]:
    """Create tags and return their IDs."""
    cursor = conn.cursor()
    tag_ids = []

    for name in TAG_NAMES[:NUM_TAGS]:
        color = random.choice(COLORS)
        style = random.choice(STYLES)
        cursor.execute(
            "INSERT INTO tags (name, color, style) VALUES (?, ?, ?)",
            (name, color, style),
        )
        tag_ids.append(cursor.lastrowid)

    conn.commit()
    return tag_ids


def create_feeds(conn: sqlite3.Connection, tag_ids: list[int]) -> list[int]:
    """Create feeds with invalid URLs and return their IDs."""
    cursor = conn.cursor()
    feed_ids = []
    now = datetime.now()

    print(f"  Creating {NUM_FEEDS} feeds...")

    for i in range(NUM_FEEDS):
        title = f"Feed #{i + 1}: {lorem_words(3).title()}"
        url = f"https://invalid-feed-{i + 1}.example.invalid/rss.xml"
        description = lorem_sentence()
        site_url = f"https://invalid-feed-{i + 1}.example.invalid"
        color = random.choice(COLORS)
        fetch_frequency = random.choice(["smart", "frequent", "normal", "rare"])
        created_at = now - timedelta(days=random.randint(0, 365))

        cursor.execute(
            """
            INSERT INTO feeds (url, title, description, site_url, color,
                               fetch_frequency, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (url, title, description, site_url, color, fetch_frequency,
             created_at.isoformat(), created_at.isoformat()),
        )
        feed_ids.append(cursor.lastrowid)

        # Assign 0-5 random tags to each feed
        num_tags = random.randint(0, 5)
        if num_tags > 0 and tag_ids:
            selected_tags = random.sample(tag_ids, min(num_tags, len(tag_ids)))
            for tag_id in selected_tags:
                cursor.execute(
                    "INSERT OR IGNORE INTO feed_tags (feed_id, tag_id) VALUES (?, ?)",
                    (feed_ids[-1], tag_id),
                )

        if (i + 1) % 100 == 0:
            print(f"    Created {i + 1}/{NUM_FEEDS} feeds")

    conn.commit()
    return feed_ids


def create_articles(conn: sqlite3.Connection, feed_ids: list[int]) -> None:
    """Create articles distributed across feeds."""
    cursor = conn.cursor()
    now = datetime.now()

    print(f"  Creating {NUM_ARTICLES} articles...")

    # Disable FTS triggers for bulk insert performance
    print("    Disabling FTS triggers...")
    cursor.execute("DROP TRIGGER IF EXISTS articles_fts_insert")
    cursor.execute("DROP TRIGGER IF EXISTS articles_fts_update")
    cursor.execute("DROP TRIGGER IF EXISTS articles_fts_delete")
    conn.commit()

    # Batch insert for performance
    batch_size = 1000
    articles_batch = []

    for i in range(NUM_ARTICLES):
        feed_id = random.choice(feed_ids)
        title = f"Article #{i + 1}: {lorem_words(random.randint(4, 10)).title()}"
        guid = str(uuid.uuid4())
        url = f"https://example.invalid/article/{guid}"
        content = lorem_paragraphs(random.randint(2, 6))
        summary = lorem_paragraph()
        author = lorem_words(2).title()
        published_at = now - timedelta(
            days=random.randint(0, 365),
            hours=random.randint(0, 23),
            minutes=random.randint(0, 59),
        )
        is_read = random.random() < 0.3  # 30% read
        is_starred = random.random() < 0.05  # 5% starred
        created_at = published_at + timedelta(minutes=random.randint(1, 60))

        articles_batch.append((
            feed_id, guid, title, url, content, summary, author,
            published_at.isoformat(), is_read, is_starred,
            created_at.isoformat(), created_at.isoformat(),
        ))

        if len(articles_batch) >= batch_size:
            cursor.executemany(
                """
                INSERT INTO articles (feed_id, guid, title, url, content, summary,
                                      author, published_at, is_read, is_starred,
                                      created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                articles_batch,
            )
            conn.commit()
            articles_batch = []
            print(f"    Created {i + 1}/{NUM_ARTICLES} articles")

    # Insert remaining articles
    if articles_batch:
        cursor.executemany(
            """
            INSERT INTO articles (feed_id, guid, title, url, content, summary,
                                  author, published_at, is_read, is_starred,
                                  created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            articles_batch,
        )
        conn.commit()

    print(f"    Created {NUM_ARTICLES}/{NUM_ARTICLES} articles")

    # Bulk populate FTS table (much faster than trigger-based)
    print("    Populating FTS index...")
    cursor.execute("""
        INSERT INTO articles_fts(rowid, article_id, title, content, summary, author)
        SELECT id, id, title, content, summary, author FROM articles
    """)
    conn.commit()

    # Recreate triggers for future inserts
    print("    Recreating FTS triggers...")
    cursor.execute("""
        CREATE TRIGGER IF NOT EXISTS articles_fts_insert AFTER INSERT ON articles BEGIN
            INSERT INTO articles_fts(rowid, article_id, title, content, summary, author)
            VALUES (new.id, new.id, new.title, new.content, new.summary, new.author);
        END
    """)
    cursor.execute("""
        CREATE TRIGGER IF NOT EXISTS articles_fts_delete AFTER DELETE ON articles BEGIN
            DELETE FROM articles_fts WHERE rowid = old.id;
        END
    """)
    cursor.execute("""
        CREATE TRIGGER IF NOT EXISTS articles_fts_update AFTER UPDATE ON articles BEGIN
            DELETE FROM articles_fts WHERE rowid = old.id;
            INSERT INTO articles_fts(rowid, article_id, title, content, summary, author)
            VALUES (new.id, new.id, new.title, new.content, new.summary, new.author);
        END
    """)
    conn.commit()


def print_stats(conn: sqlite3.Connection) -> None:
    """Print database statistics."""
    cursor = conn.cursor()

    cursor.execute("SELECT COUNT(*) FROM feeds")
    num_feeds = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM articles")
    num_articles = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM tags")
    num_tags = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM feed_tags")
    num_feed_tags = cursor.fetchone()[0]

    print("\nDatabase statistics:")
    print(f"  Feeds: {num_feeds:,}")
    print(f"  Articles: {num_articles:,}")
    print(f"  Tags: {num_tags:,}")
    print(f"  Feed-tag associations: {num_feed_tags:,}")


def main() -> None:
    # Remove existing database
    if DB_PATH.exists():
        print(f"Removing existing database at {DB_PATH}")
        DB_PATH.unlink()

    print(f"Creating stress test database at {DB_PATH}")

    conn = sqlite3.connect(DB_PATH)

    try:
        print("\nRunning migrations...")
        run_migrations(conn)

        print("\nCreating tags...")
        tag_ids = create_tags(conn)
        print(f"  Created {len(tag_ids)} tags")

        print("\nCreating feeds...")
        feed_ids = create_feeds(conn, tag_ids)

        print("\nCreating articles...")
        create_articles(conn, feed_ids)

        print_stats(conn)

        print(f"\nStress test database created successfully at {DB_PATH}")
        print("To use it, set the DATABASE_URL environment variable:")
        print(f"  export DATABASE_URL=sqlite://{DB_PATH}")

    finally:
        conn.close()


if __name__ == "__main__":
    main()
