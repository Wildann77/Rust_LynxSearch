use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

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
    let (state, _rx) = backend::AppState::test_state();
    let app = backend::create_router_with_state(state.clone());

    // 7. GET /api/folders
    let req = Request::builder()
        .uri("/api/folders")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 8. DELETE /api/folders/{id}
    let folder =
        backend::domain::models::Folder::new(std::path::PathBuf::from("/test/folder_route"));
    let folder_id = folder.id;
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

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
    let (state, _rx) = backend::AppState::test_state();
    let app = backend::create_router_with_state(state.clone());

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
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let folder_job_id = json_body["job_id"].as_str().unwrap().to_string();

    // 10. POST /api/index (200 OK)
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("src").join("lib.rs");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    std::fs::write(&file_path, "pub fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();

    let folder = backend::domain::models::Folder::new(temp_dir.path().to_path_buf());
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let body = serde_json::to_vec(&json!({
        "folder_id": *folder.id.as_uuid(),
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
    assert_eq!(res.status(), StatusCode::OK);

    // 11. GET /api/index/jobs/{id}
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{folder_job_id}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 12. POST /api/index/jobs/{id}/cancel
    let req = Request::builder()
        .uri(format!("/api/index/jobs/{folder_job_id}/cancel"))
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
    let (state, _rx) = backend::AppState::test_state();
    let app = backend::create_router_with_state(state.clone());

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let file_path = root.join("main.rs");
    std::fs::write(&file_path, "fn main() { println!(\"hello\"); }").unwrap();

    let folder = backend::domain::models::Folder::new(root.to_path_buf());
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let doc_id = backend::domain::models::DocumentId::from_relative_path(folder.id, "main.rs");

    let entry = backend::domain::models::RegistryEntry {
        id: doc_id,
        folder_id: folder.id,
        relative_path: "main.rs".to_string(),
        content_hash: "hash".to_string(),
        file_size: 100,
        status: backend::domain::models::DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    state
        .repositories
        .registry
        .upsert_entry(&entry)
        .await
        .unwrap();

    // 14. GET /api/documents/{id}
    let req = Request::builder()
        .uri(format!("/api/documents/{}", doc_id.as_uuid()))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 15. DELETE /api/documents/{id}
    let req = Request::builder()
        .uri(format!("/api/documents/{}", doc_id.as_uuid()))
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
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 18. POST /api/settings/reset
    let req = Request::builder()
        .uri("/api/settings/reset")
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json_body["weights"]["title"], 3.0);
    assert_eq!(json_body["weights"]["tags"], 2.0);
    assert_eq!(json_body["weights"]["content"], 1.0);
}
