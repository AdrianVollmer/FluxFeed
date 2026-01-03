-- Add OpenGraph metadata to articles

ALTER TABLE articles ADD COLUMN og_image TEXT;
ALTER TABLE articles ADD COLUMN og_description TEXT;
ALTER TABLE articles ADD COLUMN og_site_name TEXT;
