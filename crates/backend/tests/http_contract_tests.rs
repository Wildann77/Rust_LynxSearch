use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use backend::api::middlewares::X_REQUEST_ID;
use backend::error::{AppError, ErrorCode, ErrorResponse};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_10_6_http_contract_status_codes_and_error_bodies() {
    let app = backend::create_router();

    // 1. Success 200: GET /api/health
    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Success 200: GET /api/health/live
    let req = Request::builder()
        .uri("/api/health/live")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. 404 Not Found: Nonexistent job GET /api/index/jobs/:id
    let missing_job_id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{missing_job_id}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await.unwrap();
    let err_body: ErrorResponse =
        serde_json::from_slice(&bytes).expect("Structured ErrorResponse body");
    assert_eq!(err_body.code, ErrorCode::JobNotFound);

    // 4. 404 Not Found: Nonexistent folder DELETE /api/folders/:id
    let missing_folder_id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/folders/{missing_folder_id}"))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await.unwrap();
    let err_body: ErrorResponse =
        serde_json::from_slice(&bytes).expect("Structured ErrorResponse body");
    assert_eq!(err_body.code, ErrorCode::FolderNotFound);

    // 5. 422 Unprocessable Entity: Invalid folder registration payload
    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"root_path": ""}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await.unwrap();
    let err_body: ErrorResponse =
        serde_json::from_slice(&bytes).expect("Structured ErrorResponse body");
    assert_eq!(err_body.code, ErrorCode::ValidationFailed);
    assert!(err_body.details.is_some());

    // 6. 422 Unprocessable Entity: Path traversal component rejected by DTO validation
    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"root_path": "/home/user/../../etc/passwd"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await.unwrap();
    let err_body: ErrorResponse =
        serde_json::from_slice(&bytes).expect("Structured ErrorResponse body");
    assert_eq!(err_body.code, ErrorCode::ValidationFailed);

    // 7. 400 Bad Request / 422: Malformed UUID in path
    let req = Request::builder()
        .uri("/api/folders/not-a-valid-uuid")
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status() == StatusCode::BAD_REQUEST || res.status() == StatusCode::UNPROCESSABLE_ENTITY
    );

    // 8. 409 Conflict: Cancelling inactive / nonexistent job
    let cancel_uuid = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{cancel_uuid}/cancel"))
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status() == StatusCode::NOT_FOUND || res.status() == StatusCode::CONFLICT);

    // 9. CORS local response: Tauri origin allowed with expose x-request-id
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
    assert!(res.headers().get(&X_REQUEST_ID).is_some());

    // 10. Tracing middleware preserves custom x-request-id without breaking response
    let custom_trace_id = "test-trace-id-12345";
    let req = Request::builder()
        .uri("/api/health/live")
        .method("GET")
        .header(&X_REQUEST_ID, custom_trace_id)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(&X_REQUEST_ID).unwrap().to_str().unwrap(),
        custom_trace_id
    );
}

#[tokio::test]
async fn test_10_6_direct_app_error_status_and_body_mappings() {
    use axum::response::IntoResponse;

    // Direct mapping checks for internal taxonomy
    let cases = [
        (
            AppError::InvalidQuery("malformed syntax".into()),
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidQuery,
        ),
        (
            AppError::PathTraversal("../secret".into()),
            StatusCode::FORBIDDEN,
            ErrorCode::PathTraversalDetected,
        ),
        (
            AppError::FolderNotFound(Uuid::new_v4()),
            StatusCode::NOT_FOUND,
            ErrorCode::FolderNotFound,
        ),
        (
            AppError::JobNotFound(Uuid::new_v4()),
            StatusCode::NOT_FOUND,
            ErrorCode::JobNotFound,
        ),
        (
            AppError::DocumentNotFound(Uuid::new_v4()),
            StatusCode::NOT_FOUND,
            ErrorCode::DocumentNotFound,
        ),
        (
            AppError::JobConflict(Uuid::new_v4()),
            StatusCode::CONFLICT,
            ErrorCode::JobConflict,
        ),
        (
            AppError::ValidationFailed("field required".into()),
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::ValidationFailed,
        ),
        (
            AppError::Internal("internal crash".into()),
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::InternalServerError,
        ),
        (
            AppError::SearchEngine("es down".into()),
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::SearchEngineUnavailable,
        ),
        (
            AppError::RequestTimeout("timeout".into()),
            StatusCode::REQUEST_TIMEOUT,
            ErrorCode::RequestTimeout,
        ),
    ];

    for (err, expected_status, expected_code) in cases {
        assert_eq!(
            err.status_code(),
            expected_status,
            "Status mismatch for {expected_code:?}"
        );
        assert_eq!(
            err.error_code(),
            expected_code,
            "Code mismatch for {expected_code:?}"
        );

        let resp = err.into_response();
        assert_eq!(resp.status(), expected_status);

        let bytes = to_bytes(resp.into_body(), 1024 * 16).await.unwrap();
        let body: ErrorResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.code, expected_code);
    }
}
