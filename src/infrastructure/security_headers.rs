use axum::{
    extract::Request,
    http::header::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

/// Security headers middleware
///
/// Adds standard security headers to all responses:
/// - X-Frame-Options: Prevents clickjacking
/// - X-Content-Type-Options: Prevents MIME sniffing
/// - X-XSS-Protection: Legacy XSS protection for older browsers
/// - Referrer-Policy: Controls referrer information
/// - Content-Security-Policy: Prevents XSS and injection attacks
pub async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Prevent clickjacking
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );

    // Prevent MIME type sniffing
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );

    // Legacy XSS protection for older browsers
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );

    // Control referrer information
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Content Security Policy
    // - default-src 'self': Only allow resources from same origin
    // - script-src 'self' 'unsafe-inline': Allow scripts from same origin and inline (for HTMX)
    // - style-src 'self' 'unsafe-inline': Allow styles from same origin and inline
    // - img-src 'self' https: data:: Allow images from same origin, HTTPS sources, and data URIs
    // - font-src 'self': Only fonts from same origin
    // - connect-src 'self': Only AJAX/fetch to same origin
    // - frame-ancestors 'none': Prevent framing (like X-Frame-Options)
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' https: data:; \
             font-src 'self'; \
             connect-src 'self'; \
             frame-ancestors 'none'",
        ),
    );

    response
}
