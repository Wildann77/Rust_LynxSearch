use axum::{Router, routing::get};

pub fn create_router() -> Router {
    Router::new().route("/api/health", get(|| async { "{\"status\":\"ok\"}" }))
}
