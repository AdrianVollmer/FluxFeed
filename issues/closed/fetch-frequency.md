# Fetch Frequency

All feeds have a fetch frequency, which is adaptive by default.

The user should be able to edit a feed and change the fetch frequency.
Allowed values must be a positive, integer representing the hours
between fetching.

In adaptive fetching, we use information from the server to adjust the
fetch frequency. Start with the RSS feed's `ttl` field.

Also, make sure the AVOID the following behaviors:

- Loading the feed way too often, like every 10 seconds, when it never
  updates anywhere near that often
- Not using If-Modified-Since/If-None-Match, and so always download a
  full copy of the feed even when nothing has changed
- Scraping individual posts after pulling a copy of the feed even though
  they have the same damn content
- Sending referrers which make no sense is just bad manners.

When adapting, the resulting frequency should always be between 1 hour
and 1 week.
