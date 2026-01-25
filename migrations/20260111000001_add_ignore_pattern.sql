-- Add ignore_pattern column to feeds table
-- This regex pattern is matched against article titles during ingestion
-- Articles with matching titles will not be stored
ALTER TABLE feeds ADD COLUMN ignore_pattern TEXT;
