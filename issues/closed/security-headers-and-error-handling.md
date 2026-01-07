# Add security headers and sanitize error messages

## Problem

The application is missing standard security headers and exposes verbose error messages that could leak internal details to attackers.

## Files affected

- `src/main.rs` - needs security header middleware
- `src/api/manual_fetch.rs` - exposes raw error messages
- `src/api/feeds.rs` - exposes fetch error details

## Issues

### 1. Missing security headers

No security headers are set on responses. Should add:
- `Content-Security-Policy` - prevent XSS and data injection
- `X-Frame-Options: DENY` - prevent clickjacking
- `X-Content-Type-Options: nosniff` - prevent MIME sniffing
- `Referrer-Policy: strict-origin-when-cross-origin`
- `X-XSS-Protection: 1; mode=block` (legacy browser support)

### 2. Verbose error messages

`src/api/manual_fetch.rs:30`:
```rust
message: format!("Fetch failed: {}", e),  // Leaks internal error
```

`src/api/feeds.rs:312-313`:
```rust
format!("Unable to fetch the feed: {}", msg),  // Leaks fetch details
```

These could expose:
- Internal hostnames/IPs
- Library versions
- File paths
- Database details

### 3. Debug logging by default

`src/main.rs:40` defaults to debug level:
```rust
.unwrap_or_else(|_| "fluxfeed=debug,tower_http=debug".into())
```

## Proposed solution

1. Add security headers middleware in `src/main.rs`:
   ```rust
   use tower_http::set_header::SetResponseHeaderLayer;
   use http::header;

   .layer(SetResponseHeaderLayer::if_not_present(
       header::X_FRAME_OPTIONS,
       HeaderValue::from_static("DENY"),
   ))
   // ... etc
   ```

2. Replace verbose errors with generic messages:
   ```rust
   // Before
   format!("Fetch failed: {}", e)
   // After
   "Feed fetch failed. Check the URL and try again.".to_string()
   ```
   Log the actual error server-side for debugging.

3. Change default log level from `debug` to `info`
