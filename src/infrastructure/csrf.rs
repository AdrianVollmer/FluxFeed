use axum::{
    extract::Request,
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;

const CSRF_COOKIE_NAME: &str = "csrf_token";
const CSRF_HEADER_NAME: &str = "x-csrf-token";
const TOKEN_LENGTH: usize = 32;

/// Generate a new CSRF token
fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_LENGTH];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Extract CSRF token from cookie header
fn get_token_from_cookie(req: &Request) -> Option<String> {
    req.headers()
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let cookie = cookie.trim();
            if let Some(value) = cookie.strip_prefix(CSRF_COOKIE_NAME) {
                value.strip_prefix('=').map(|v| v.to_string())
            } else {
                None
            }
        })
}

/// Extract CSRF token from request header
fn get_token_from_header(req: &Request) -> Option<String> {
    req.headers()
        .get(CSRF_HEADER_NAME)?
        .to_str()
        .ok()
        .map(|s| s.to_string())
}

/// Check if this is a state-changing request that needs CSRF validation
fn needs_csrf_validation(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::DELETE | Method::PATCH
    )
}

/// CSRF protection middleware
///
/// This middleware implements the double-submit cookie pattern:
/// 1. Sets a CSRF token cookie if not present
/// 2. On state-changing requests (POST, PUT, DELETE, PATCH), validates that
///    the X-CSRF-Token header matches the cookie value
pub async fn csrf_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let cookie_token = get_token_from_cookie(&req);

    // For state-changing requests, validate the CSRF token
    if needs_csrf_validation(&method) {
        let header_token = get_token_from_header(&req);

        match (&cookie_token, &header_token) {
            (Some(cookie), Some(header)) if cookie == header => {
                // Token is valid, continue
            }
            (None, _) => {
                tracing::warn!("CSRF validation failed: no cookie token");
                return (
                    StatusCode::FORBIDDEN,
                    "CSRF validation failed: missing token cookie",
                )
                    .into_response();
            }
            (_, None) => {
                tracing::warn!("CSRF validation failed: no header token");
                return (
                    StatusCode::FORBIDDEN,
                    "CSRF validation failed: missing token header",
                )
                    .into_response();
            }
            (Some(_), Some(_)) => {
                tracing::warn!("CSRF validation failed: token mismatch");
                return (
                    StatusCode::FORBIDDEN,
                    "CSRF validation failed: token mismatch",
                )
                    .into_response();
            }
        }
    }

    // Process the request
    let mut response = next.run(req).await;

    // Set the CSRF cookie if not present
    // Note: NOT HttpOnly so that JavaScript can read it for HTMX requests
    // SameSite=Strict provides protection against CSRF from other origins
    if cookie_token.is_none() {
        let new_token = generate_token();
        let cookie_value = format!(
            "{}={}; Path=/; SameSite=Strict",
            CSRF_COOKIE_NAME, new_token
        );
        if let Ok(header_value) = cookie_value.parse() {
            response
                .headers_mut()
                .insert(header::SET_COOKIE, header_value);
        }
    }

    response
}
