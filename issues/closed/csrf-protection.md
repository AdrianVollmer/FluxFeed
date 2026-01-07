# Implement CSRF protection for state-changing endpoints

## Problem

The application has no CSRF (Cross-Site Request Forgery) protection. All POST, PUT, and DELETE endpoints can be triggered by malicious third-party websites if a user is authenticated, allowing attackers to:
- Create/delete/modify feeds
- Create/delete/modify groups
- Mark articles as read/starred
- Trigger feed fetches

## Files affected

- `src/main.rs` - needs CSRF middleware layer
- `src/web/templates/feeds/form.html` - needs CSRF token field
- `src/web/templates/feeds/edit_form.html` - needs CSRF token field
- `src/web/templates/feeds/import_form.html` - needs CSRF token field
- `src/web/templates/feeds/detail.html` - needs CSRF token for hx-post/hx-delete
- `src/web/templates/groups/form.html` - needs CSRF token field
- `src/web/templates/groups/assign_feed.html` - needs CSRF token for hx-put
- `src/web/templates/articles/list.html` - needs CSRF token for mark-all-read
- `src/web/templates/articles/_action_icons.html` - needs CSRF token for toggle actions
- `Cargo.toml` - needs session/CSRF crate dependency

## Current state

No CSRF middleware or token validation exists. The only middleware layers are:
```rust
.layer(CompressionLayer::new())
.layer(TraceLayer::new_for_http())
```

Forms and HTMX requests have no token fields:
```html
<form hx-post="/feeds" ...>
    <!-- No CSRF token -->
    <input type="text" name="url" ...>
</form>
```

## Proposed solution

1. Add `tower-sessions` and a CSRF crate to `Cargo.toml`
2. Generate CSRF tokens per session and pass them to templates
3. Add hidden `<input type="hidden" name="_csrf" value="...">` to all forms
4. Configure HTMX to send tokens via headers globally:
   ```html
   <body hx-headers='{"X-CSRF-Token": "{{ csrf_token }}"}'>
   ```
5. Add middleware to validate tokens on all non-GET requests
