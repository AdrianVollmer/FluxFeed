-- Add column to store TTL extracted from RSS feeds
ALTER TABLE feeds ADD COLUMN ttl_minutes INTEGER;

-- Update existing "smart" values to "adaptive" for clarity
UPDATE feeds SET fetch_frequency = 'adaptive' WHERE fetch_frequency = 'smart';
