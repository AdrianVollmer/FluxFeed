-- Full-text search for articles

CREATE VIRTUAL TABLE IF NOT EXISTS articles_fts USING fts5(
    article_id UNINDEXED,
    title,
    content,
    summary,
    author
);

-- Triggers to keep FTS index in sync
CREATE TRIGGER IF NOT EXISTS articles_fts_insert AFTER INSERT ON articles BEGIN
    INSERT INTO articles_fts(rowid, article_id, title, content, summary, author)
    VALUES (new.id, new.id, new.title, new.content, new.summary, new.author);
END;

CREATE TRIGGER IF NOT EXISTS articles_fts_delete AFTER DELETE ON articles BEGIN
    DELETE FROM articles_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS articles_fts_update AFTER UPDATE ON articles BEGIN
    DELETE FROM articles_fts WHERE rowid = old.id;
    INSERT INTO articles_fts(rowid, article_id, title, content, summary, author)
    VALUES (new.id, new.id, new.title, new.content, new.summary, new.author);
END;
