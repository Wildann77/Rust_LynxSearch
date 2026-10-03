use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use backend::api::dtos::{HealthSummaryResponseDto, LivenessResponseDto, ReadinessResponseDto};
use tower::ServiceExt;

#[tokio::test]
async fn test_health_summary_endpoint() {
    let app = backend::create_router();

    let request = Request::builder()
        .uri("/api/health")
        .method("GET")
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    let summary: HealthSummaryResponseDto = serde_json::from_slice(&body_bytes)
        .expect("Failed to deserialize HealthSummaryResponseDto");

    assert!(summary.status == "ok" || summary.status == "degraded");
    assert_eq!(summary.version, env!("CARGO_PKG_VERSION"));
    assert!(!summary.timestamp.is_empty());
    assert!(summary.database.status == "up" || summary.database.status == "down");
    assert!(summary.elasticsearch.status == "up" || summary.elasticsearch.status == "down");
}

#[tokio::test]
async fn test_health_liveness_endpoint() {
    let app = backend::create_router();

    let request = Request::builder()
        .uri("/api/health/live")
        .method("GET")
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    let liveness: LivenessResponseDto =
        serde_json::from_slice(&body_bytes).expect("Failed to deserialize LivenessResponseDto");

    assert_eq!(liveness.status, "alive");
}

#[tokio::test]
async fn test_health_readiness_endpoint() {
    let app = backend::create_router();

    let request = Request::builder()
        .uri("/api/health/ready")
        .method("GET")
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    // In unit/integration tests without running Docker containers, readiness should return 503
    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    let readiness: ReadinessResponseDto =
        serde_json::from_slice(&body_bytes).expect("Failed to deserialize ReadinessResponseDto");

    if status == StatusCode::OK {
        assert_eq!(readiness.status, "ready");
    } else {
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(readiness.status, "unavailable");
        assert!(readiness.database.is_some());
        assert!(readiness.elasticsearch.is_some());
    }
}
