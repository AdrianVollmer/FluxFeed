# Implement SSRF protection for feed URL fetching

## Problem

The application fetches RSS feeds from user-provided URLs without validating against private/internal IP ranges. Attackers can exploit this to:
- Access cloud metadata endpoints (e.g., `http://169.254.169.254/latest/meta-data/` on AWS) to steal credentials
- Scan internal networks and services
- Access internal admin interfaces
- Exfiltrate data via error messages or timing attacks

## Files affected

- `src/api/feeds.rs` - URL validation in `create_feed()` and `import_feeds()`
- `src/infrastructure/rss_fetcher.rs` - HTTP client making the requests

## Current state

URL validation only checks for http/https prefix (`src/api/feeds.rs:33-38`):

```rust
if !url.starts_with("http://") && !url.starts_with("https://") {
    return Err(FeedServiceError::InvalidUrl(...))
}
```

No blocking of:
- Loopback: `127.0.0.0/8`, `::1`
- Private networks: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`
- Link-local: `169.254.0.0/16`, `fe80::/10`
- Cloud metadata: `169.254.169.254`

## Proposed solution

1. Add a URL validation function that:
   - Parses the URL and extracts the hostname
   - Resolves DNS to get the IP address
   - Checks IP against blocklist of private ranges
   - Rejects URLs pointing to internal resources

2. Apply validation in:
   - `create_feed()` before storing the URL
   - `import_feeds()` for bulk imports
   - Optionally in `rss_fetcher.rs` as defense-in-depth

3. Example validation:
   ```rust
   fn is_private_ip(ip: IpAddr) -> bool {
       match ip {
           IpAddr::V4(v4) => {
               v4.is_loopback() ||
               v4.is_private() ||
               v4.is_link_local() ||
               v4.octets()[..2] == [169, 254]  // AWS metadata
           }
           IpAddr::V6(v6) => v6.is_loopback(),
       }
   }
   ```

4. Consider DNS rebinding attacks - validate IP at fetch time, not just at creation time
