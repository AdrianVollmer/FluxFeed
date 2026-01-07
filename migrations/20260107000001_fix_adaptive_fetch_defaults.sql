-- Fix existing feeds with old 'smart' value (should be 'adaptive')
UPDATE feeds SET fetch_frequency = 'adaptive' WHERE fetch_frequency = 'smart';

-- Fix feeds with fetch_interval below 60 minutes (1 hour minimum for adaptive)
UPDATE feeds SET fetch_interval_minutes = 60
WHERE fetch_frequency = 'adaptive' AND fetch_interval_minutes < 60;
