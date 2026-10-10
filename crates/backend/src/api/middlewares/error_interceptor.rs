use axum::{
    Json,
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::error::{ErrorCode, ErrorResponse};

pub async fn ensure_structured_errors(req: Request, next: Next) -> Response {
    let res = next.run(req).await;

    let status = res.status();
    if !status.is_client_error() && !status.is_server_error() {
        return res;
    }

    // Preserve existing structured json response (e.g. from AppError or Validated extractors)
    let is_json = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("application/json"));

    if is_json {
        return res;
    }

    let (code, message) = if status == StatusCode::REQUEST_TIMEOUT {
        (
            ErrorCode::RequestTimeout,
            "Request execution exceeded timeout limit.".to_string(),
        )
    } else if status == StatusCode::NOT_FOUND {
        (
            ErrorCode::DocumentNotFound,
            "Endpoint not found.".to_string(),
        )
    } else if status == StatusCode::METHOD_NOT_ALLOWED {
        (
            ErrorCode::InvalidQuery,
            "HTTP method not allowed for this endpoint.".to_string(),
        )
    } else {
        (
            ErrorCode::InternalServerError,
            format!("An unexpected error occurred (HTTP {})", status.as_u16()),
        )
    };

    let body = ErrorResponse {
        code,
        message,
        details: None,
    };

    tracing::warn!(
        error_code = code.as_str(),
        status = status.as_u16(),
        "HTTP error response intercepted and structured"
    );

    let mut response = (status, Json(body)).into_response();
    for (k, v) in res.headers() {
        if k != header::CONTENT_TYPE && k != header::CONTENT_LENGTH {
            response.headers_mut().insert(k.clone(), v.clone());
        }
    }
    response
}
