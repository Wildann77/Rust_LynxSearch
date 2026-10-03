use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_health_routes() {
    let app = backend::create_router();

    // 1. GET /api/health
    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. GET /api/health/live
    let req = Request::builder()
        .uri("/api/health/live")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. GET /api/health/ready
    let req = Request::builder()
        .uri("/api/health/ready")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status() == StatusCode::OK || res.status() == StatusCode::SERVICE_UNAVAILABLE);

    // 4. GET /api/stats
    let req = Request::builder()
        .uri("/api/stats")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_search_and_suggest_routes() {
    let app = backend::create_router();

    // 5. GET /api/search without params
    let req = Request::builder()
        .uri("/api/search")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // GET /api/search with valid params
    let req = Request::builder()
        .uri("/api/search?q=rust&page=2&size=10")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. GET /api/suggest
    let req = Request::builder()
        .uri("/api/suggest?q=lynx")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_folder_routes() {
    let app = backend::create_router();

    // 7. GET /api/folders
    let req = Request::builder()
        .uri("/api/folders")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 8. DELETE /api/folders/{id}
    let folder_id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/folders/{folder_id}"))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_index_and_job_routes() {
    let app = backend::create_router();

    // 9. POST /api/index/folder (202 Accepted)
    #[cfg(unix)]
    let valid_path = "/home/developer/code";
    #[cfg(windows)]
    let valid_path = "C:\\code";

    let body = serde_json::to_vec(&json!({ "root_path": valid_path })).unwrap();
    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::ACCEPTED);

    // 10. POST /api/index (202 Accepted)
    let body = serde_json::to_vec(&json!({
        "folder_id": Uuid::new_v4(),
        "relative_path": "src/lib.rs"
    }))
    .unwrap();
    let req = Request::builder()
        .uri("/api/index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::ACCEPTED);

    // 11. GET /api/index/jobs/{id}
    let job_id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{job_id}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 12. POST /api/index/jobs/{id}/cancel
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{job_id}/cancel"))
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 13. POST /api/index/rebuild (202 Accepted)
    let req = Request::builder()
        .uri("/api/index/rebuild")
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_document_routes() {
    let app = backend::create_router();
    let doc_id = Uuid::new_v4();

    // 14. GET /api/documents/{id}
    let req = Request::builder()
        .uri(format!("/api/documents/{doc_id}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 15. DELETE /api/documents/{id}
    let req = Request::builder()
        .uri(format!("/api/documents/{doc_id}"))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_settings_routes() {
    let app = backend::create_router();

    // 16. GET /api/settings
    let req = Request::builder()
        .uri("/api/settings")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 17. PUT /api/settings
    let body = serde_json::to_vec(&json!({
        "max_file_size_bytes": 4194304,
        "weights": {
            "title": 4.0,
            "tags": 2.5,
            "content": 1.0
        },
        "ignore_patterns": [".git", "node_modules", "target"]
    }))
    .unwrap();
    let req = Request::builder()
        .uri("/api/settings")
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
