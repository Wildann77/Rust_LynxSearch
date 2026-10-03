use axum::http::{HeaderName, HeaderValue, Method, header};
use tower_http::cors::{AllowOrigin, CorsLayer};

pub fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _parts| {
            if let Ok(s) = origin.to_str() {
                s == "tauri://localhost"
                    || s == "https://tauri.localhost"
                    || s.starts_with("http://localhost:")
                    || s.starts_with("http://127.0.0.1:")
                    || s == "http://localhost"
                    || s == "http://127.0.0.1"
            } else {
                false
            }
        }))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::AUTHORIZATION,
            HeaderName::from_static("x-request-id"),
        ])
        .expose_headers([HeaderName::from_static("x-request-id")])
        .allow_credentials(true)
}
