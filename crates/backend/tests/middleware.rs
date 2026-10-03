use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use axum::routing::get;
use backend::api::middlewares::{X_REQUEST_ID, apply_middlewares};
use backend::{ErrorCode, ErrorResponse};
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
async fn test_request_id_generated_when_missing() {
    let app = backend::create_router();

    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req_id = res.headers().get(&X_REQUEST_ID);
    assert!(
        req_id.is_some(),
        "Response must include x-request-id header"
    );

    let req_id_str = req_id.unwrap().to_str().unwrap();
    assert!(
        uuid::Uuid::parse_str(req_id_str).is_ok(),
        "Generated x-request-id must be valid UUID"
    );
}

#[tokio::test]
async fn test_request_id_preserved_when_provided() {
    let app = backend::create_router();
    let client_req_id = "custom-client-trace-id-998877";

    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .header(&X_REQUEST_ID, client_req_id)
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let resp_req_id = res.headers().get(&X_REQUEST_ID).unwrap().to_str().unwrap();
    assert_eq!(resp_req_id, client_req_id);
}

#[tokio::test]
async fn test_cors_allowed_origins_and_preflight() {
    let app = backend::create_router();

    // 1. Tauri desktop protocol origin
    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .header(header::ORIGIN, "tauri://localhost")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        "tauri://localhost"
    );
    assert_eq!(
        res.headers()
            .get(header::ACCESS_CONTROL_EXPOSE_HEADERS)
            .unwrap(),
        "x-request-id"
    );

    // 2. Vite dev localhost origin
    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .header(header::ORIGIN, "http://localhost:1420")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        "http://localhost:1420"
    );

    // 3. Disallowed remote origin
    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .header(header::ORIGIN, "https://malicious.evil.com")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none(),
        "Malicious origin must not receive Access-Control-Allow-Origin"
    );

    // 4. Preflight OPTIONS request
    let preflight_req = Request::builder()
        .uri("/api/folders")
        .method(Method::OPTIONS)
        .header(header::ORIGIN, "tauri://localhost")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(
            header::ACCESS_CONTROL_REQUEST_HEADERS,
            "content-type,x-request-id",
        )
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(preflight_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        "tauri://localhost"
    );
}

#[tokio::test]
async fn test_selective_compression_policy() {
    let app = backend::create_router();

    // 1. Heavy route: /api/stats (must compress with gzip when client requests)
    let req = Request::builder()
        .uri("/api/stats")
        .method("GET")
        .header(header::ACCEPT_ENCODING, "gzip")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::CONTENT_ENCODING)
            .map(|h| h.to_str().unwrap()),
        Some("gzip"),
        "Heavy route /api/stats must return gzip Content-Encoding"
    );

    // 2. Heavy route: /api/search (must compress with gzip)
    let req = Request::builder()
        .uri("/api/search?q=test")
        .method("GET")
        .header(header::ACCEPT_ENCODING, "gzip")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::CONTENT_ENCODING)
            .map(|h| h.to_str().unwrap()),
        Some("gzip"),
        "Heavy route /api/search must return gzip Content-Encoding"
    );

    // 3. Lightweight route: /api/health (must NOT have gzip compression layer)
    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .header(header::ACCEPT_ENCODING, "gzip")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers().get(header::CONTENT_ENCODING).is_none(),
        "Lightweight route /api/health should not have gzip Content-Encoding"
    );
}

#[tokio::test]
async fn test_error_layer_preserves_structured_errors_without_generic_string() {
    let app = backend::create_router();

    // 1. Validation error on /api/index/folder (path traversal returns 422 with details)
    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"root_path":"../../../etc/passwd"}"#))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_resp: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(err_resp.code, ErrorCode::ValidationFailed);
    assert!(err_resp.details.is_some(), "Details must be preserved");

    // 2. Method Not Allowed (405) turns into structured JSON ErrorResponse
    let req = Request::builder()
        .uri("/api/folders")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{}"#))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_resp: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(err_resp.code, ErrorCode::InvalidQuery);
    assert_eq!(
        err_resp.message,
        "HTTP method not allowed for this endpoint."
    );

    // 3. Unmapped 404 route must return uniform JSON ErrorResponse, not plain string
    let req = Request::builder()
        .uri("/api/non-existent-endpoint-test")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_resp: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(err_resp.code, ErrorCode::DocumentNotFound);
    assert_eq!(err_resp.message, "Endpoint not found.");
}

#[tokio::test]
async fn test_timeout_policy_returns_structured_408() {
    let slow_router = axum::Router::new().route(
        "/slow",
        get(|| async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            "finished"
        }),
    );
    let app = apply_middlewares(slow_router, Duration::from_millis(5));

    let req = Request::builder()
        .uri("/slow")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::REQUEST_TIMEOUT);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_resp: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(err_resp.code, ErrorCode::RequestTimeout);
    assert_eq!(
        err_resp.message,
        "Request execution exceeded timeout limit."
    );
}
