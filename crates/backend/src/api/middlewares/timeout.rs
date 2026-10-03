use axum::http::StatusCode;
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

pub const DEFAULT_TIMEOUT_DURATION: Duration = Duration::from_secs(30);

pub fn create_timeout_layer(duration: Duration) -> TimeoutLayer {
    TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, duration)
}
