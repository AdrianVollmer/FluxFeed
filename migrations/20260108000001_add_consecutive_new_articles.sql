-- Add column to track consecutive fetches with new articles for adaptive mode
ALTER TABLE feeds ADD COLUMN consecutive_new_articles INTEGER NOT NULL DEFAULT 0;
