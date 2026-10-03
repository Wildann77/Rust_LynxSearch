use axum::{
    Json, Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use backend::api::dtos::{PathUuid, RegisterFolderRequestDto, SearchRequestDto, ValidatedUuid};
use backend::api::extractors::{ValidatedJson, ValidatedPath, ValidatedQuery};
use backend::error::{ErrorCode, ErrorResponse};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

fn test_router() -> Router {
    Router::new()
        .route(
            "/test/search",
            get(
                |ValidatedQuery(params): ValidatedQuery<SearchRequestDto>| async move {
                    Json(json!({
                        "query": params.normalized_query(),
                        "page": params.page(),
                        "size": params.size(),
                    }))
                },
            ),
        )
        .route(
            "/test/folder",
            post(
                |ValidatedJson(payload): ValidatedJson<RegisterFolderRequestDto>| async move {
                    Json(json!({
                        "root_path": payload.normalized_path(),
                    }))
                },
            ),
        )
        .route(
            "/test/uuid-param/{id}",
            get(|ValidatedPath(path): ValidatedPath<PathUuid>| async move {
                Json(json!({
                    "id": path.id.to_string(),
                }))
            }),
        )
        .route(
            "/test/transparent-uuid/{id}",
            get(
                |ValidatedPath(id): ValidatedPath<ValidatedUuid>| async move {
                    Json(json!({
                        "id": id.to_string(),
                    }))
                },
            ),
        )
}

#[tokio::test]
async fn test_search_query_valid_request() {
    let app = test_router();

    let request = Request::builder()
        .uri("/test/search?q=%20%20rust%20search%20%20&page=2&size=50")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["query"], "rust search");
    assert_eq!(body["page"], 2);
    assert_eq!(body["size"], 50);
}

#[tokio::test]
async fn test_search_query_empty_q_contract() {
    let app = test_router();

    let request = Request::builder()
        .uri("/test/search?q=%20%20%20&page=1")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["query"], Value::Null);
    assert_eq!(body["page"], 1);
}

#[tokio::test]
async fn test_search_query_validation_failures_return_422() {
    let app = test_router();

    // 1. Page out of range (< 1)
    let request = Request::builder()
        .uri("/test/search?page=0")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.details.unwrap().get("page").is_some());

    // 2. Size out of range (> 100)
    let request = Request::builder()
        .uri("/test/search?size=101")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.details.unwrap().get("size").is_some());

    // 3. Query string deserialization failure (type mismatch)
    let request = Request::builder()
        .uri("/test/search?page=not_a_number")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error.code, ErrorCode::ValidationFailed);

    // 4. Query length > 500 characters after normalization
    let long_q = "x".repeat(501);
    let request = Request::builder()
        .uri(format!("/test/search?q={long_q}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.details.unwrap().get("q").is_some());
}

#[tokio::test]
async fn test_folder_json_valid_request() {
    let app = test_router();

    #[cfg(unix)]
    let path = "/home/developer/projects/LynxSearch";
    #[cfg(windows)]
    let path = "C:\\projects\\LynxSearch";

    let request = Request::builder()
        .uri("/test/folder")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({ "root_path": path }).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["root_path"], path);
}

#[tokio::test]
async fn test_folder_json_validation_failures_return_422() {
    let app = test_router();

    // 1. Path traversal attempt
    #[cfg(unix)]
    let bad_path = "/home/developer/../etc/passwd";
    #[cfg(windows)]
    let bad_path = "C:\\projects\\..\\windows\\system32";

    let request = Request::builder()
        .uri("/test/folder")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({ "root_path": bad_path }).to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.details.unwrap().get("root_path").is_some());

    // 2. Relative path attempt
    let request = Request::builder()
        .uri("/test/folder")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({ "root_path": "relative/path" }).to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 3. Malformed JSON syntax
    let request = Request::builder()
        .uri("/test/folder")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from("{ malformed json }"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.details.unwrap().get("_").is_some());
}

#[tokio::test]
async fn test_path_uuid_extractor_validation() {
    let app = test_router();
    let valid_uuid = Uuid::new_v4();

    // 1. Valid UUID
    let request = Request::builder()
        .uri(format!("/test/uuid-param/{valid_uuid}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["id"], valid_uuid.to_string());

    // 2. Invalid UUID in path -> returns 422 VALIDATION_FAILED
    let request = Request::builder()
        .uri("/test/uuid-param/not-a-valid-uuid")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.details.unwrap().get("path").is_some());

    // 3. ValidatedUuid transparent wrapper
    let request = Request::builder()
        .uri(format!("/test/transparent-uuid/{valid_uuid}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 4. Invalid UUID with transparent wrapper -> 422
    let request = Request::builder()
        .uri("/test/transparent-uuid/12345")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
