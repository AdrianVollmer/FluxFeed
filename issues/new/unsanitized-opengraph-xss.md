# Unsanitized OpenGraph Metadata (XSS Vulnerability)

## Severity: HIGH

## Category: Security (XSS)

## Location
- `src/infrastructure/scheduler.rs:472-495`
- `src/web/templates/articles/article_row.html:103`
- `src/infrastructure/repository.rs:356-384`

## Description

OpenGraph metadata (og_description, og_site_name, og_image) is fetched from external URLs and stored in the database without sanitization. While article content and summaries are sanitized using `ammonia::clean()`, OpenGraph data is not.

## Vulnerable Code Flow

1. **Fetching** (`scheduler.rs:472-495`):
   ```rust
   async fn extract_opengraph_from_url(url_str: &str)
       -> (Option<String>, Option<String>, Option<String>) {
       match webpage::Webpage::from_url(url_str, webpage::WebpageOptions::default()) {
           Ok(webpage) => {
               let og_description = webpage.html.opengraph.properties
                   .get("og:description").cloned(); // NOT SANITIZED
               // ...
           }
       }
   }
   ```

2. **Storage** (`repository.rs:356-384`):
   ```rust
   pub async fn update_article_opengraph(
       pool: &SqlitePool,
       article_id: i64,
       og_image: Option<String>,      // NOT SANITIZED
       og_description: Option<String>, // NOT SANITIZED
       og_site_name: Option<String>,   // NOT SANITIZED
   )
   ```

3. **Display** (`article_row.html:103`):
   ```html
   {{ item.article.og_description.as_ref().unwrap() }}
   ```

   Note: This does NOT use the `|safe` filter, so it's HTML-escaped. However, if a template mistakenly uses `|safe`, it would be vulnerable.

## Attack Scenario

A malicious website could set:
```html
<meta property="og:description" content="<script>alert('XSS')</script>">
```

When FluxFeed fetches this article, the malicious script gets stored in the database. If any template renders it without escaping (or if og_image contains a malicious URL), XSS occurs.

## Impact

- **Stored XSS**: Malicious scripts persist in database
- **User compromise**: Scripts execute in victim's browser
- **Session hijacking**: Cookies could be stolen
- **Phishing**: Malicious content displayed to users

## Solution

1. Sanitize all OpenGraph data before storage:
   ```rust
   async fn extract_opengraph_from_url(url_str: &str)
       -> (Option<String>, Option<String>, Option<String>) {
       match webpage::Webpage::from_url(url_str, webpage::WebpageOptions::default()) {
           Ok(webpage) => {
               let og_description = webpage.html.opengraph.properties
                   .get("og:description")
                   .map(|s| ammonia::clean(s)); // SANITIZE
               let og_site_name = webpage.html.opengraph.properties
                   .get("og:site_name")
                   .map(|s| ammonia::clean(s)); // SANITIZE
               // Validate og_image is a valid URL
               // ...
           }
       }
   }
   ```

2. Validate og_image URLs to ensure they start with http:// or https://

3. Add Content Security Policy (CSP) headers to prevent inline script execution

4. Audit all templates to ensure `|safe` is only used on sanitized content

## Testing

1. Create a test feed with malicious OpenGraph tags
2. Verify malicious content is sanitized
3. Add integration tests for XSS prevention
