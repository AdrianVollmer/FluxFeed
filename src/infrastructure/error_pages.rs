use askama::Template;
use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
};

use crate::web::templates::ErrorTemplate;

/// Error page middleware
///
/// This middleware intercepts error responses (4xx and 5xx status codes) that have
/// plain text bodies and renders them as proper HTML error pages using the
/// ErrorTemplate.
pub async fn error_page_middleware(req: Request, next: Next) -> Response {
    let response = next.run(req).await;

    // Only process error responses
    let status = response.status();
    if !status.is_client_error() && !status.is_server_error() {
        return response;
    }

    // Check if response is already HTML - if so, don't modify it
    if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
        if let Ok(ct) = content_type.to_str() {
            if ct.contains("text/html") {
                return response;
            }
        }
    }

    // For non-HTML error responses, render a nice error page
    // Preserve important headers from the original response
    let (parts, _body) = response.into_parts();

    let mut error_response = render_error_page(parts.status);

    // Copy over any cookies or other important headers
    for (name, value) in parts.headers.iter() {
        if name == header::SET_COOKIE {
            error_response
                .headers_mut()
                .insert(name.clone(), value.clone());
        }
    }

    error_response
}

/// Render an error page with the given status code
fn render_error_page(status: StatusCode) -> Response {
    let status_code = status.as_u16();
    let status_text = get_status_text(status);
    let message = get_default_message(status);

    let template = ErrorTemplate {
        status_code,
        status_text,
        message,
    };

    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(err) => {
            tracing::error!("Error rendering error template: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}

/// Get a human-readable status text for common HTTP status codes
fn get_status_text(status: StatusCode) -> String {
    match status {
        StatusCode::BAD_REQUEST => "Bad Request".to_string(),
        StatusCode::UNAUTHORIZED => "Unauthorized".to_string(),
        StatusCode::FORBIDDEN => "Forbidden".to_string(),
        StatusCode::NOT_FOUND => "Not Found".to_string(),
        StatusCode::METHOD_NOT_ALLOWED => "Method Not Allowed".to_string(),
        StatusCode::CONFLICT => "Conflict".to_string(),
        StatusCode::UNPROCESSABLE_ENTITY => "Unprocessable Entity".to_string(),
        StatusCode::TOO_MANY_REQUESTS => "Too Many Requests".to_string(),
        StatusCode::INTERNAL_SERVER_ERROR => "Internal Server Error".to_string(),
        StatusCode::BAD_GATEWAY => "Bad Gateway".to_string(),
        StatusCode::SERVICE_UNAVAILABLE => "Service Unavailable".to_string(),
        StatusCode::GATEWAY_TIMEOUT => "Gateway Timeout".to_string(),
        _ => status.canonical_reason().unwrap_or("Error").to_string(),
    }
}

/// Get a default user-friendly message for common HTTP status codes
fn get_default_message(status: StatusCode) -> String {
    match status {
        StatusCode::BAD_REQUEST => "The request could not be understood.".to_string(),
        StatusCode::UNAUTHORIZED => "You need to be logged in to access this.".to_string(),
        StatusCode::FORBIDDEN => "You don't have permission to access this resource.".to_string(),
        StatusCode::NOT_FOUND => "The page you're looking for doesn't exist.".to_string(),
        StatusCode::METHOD_NOT_ALLOWED => "This action is not allowed.".to_string(),
        StatusCode::CONFLICT => "There was a conflict with your request.".to_string(),
        StatusCode::TOO_MANY_REQUESTS => "Too many requests. Please try again later.".to_string(),
        StatusCode::INTERNAL_SERVER_ERROR => {
            "Something went wrong on our end. Please try again later.".to_string()
        }
        StatusCode::BAD_GATEWAY => "Unable to reach the upstream server.".to_string(),
        StatusCode::SERVICE_UNAVAILABLE => {
            "The service is temporarily unavailable. Please try again later.".to_string()
        }
        _ => "An error occurred. Please try again.".to_string(),
    }
}
