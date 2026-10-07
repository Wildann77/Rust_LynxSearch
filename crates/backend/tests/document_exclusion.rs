use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::routes::create_router_with_state;
use backend::domain::models::{DocumentId, DocumentStatus, Folder};
use backend::state::AppState;
use serde_json::{Value, json};
use std::fs;
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test]
async fn test_single_document_exclusion_and_unexclusion_flow() {
    let (state, mut rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());
    let orchestrator = state.orchestrator();

    // 1. Setup temporary directory with test files
    let temp = tempdir().unwrap();
    let root = temp.path();

    let doc_a_path = root.join("file_a.rs");
    fs::write(&doc_a_path, "pub fn func_a() -> i32 { 10 }").unwrap();

    let doc_b_path = root.join("file_b.rs");
    fs::write(&doc_b_path, "pub fn func_b() -> i32 { 20 }").unwrap();

    let folder = Folder::new(root.to_path_buf());
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // 2. Initial scan of the folder
    let _job_id = orchestrator
        .submit_index_folder(folder.id, false)
        .await
        .unwrap();
    let cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd).await.unwrap();

    let doc_a_id = DocumentId::from_relative_path(folder.id, "file_a.rs");
    let doc_b_id = DocumentId::from_relative_path(folder.id, "file_b.rs");

    // Verify initial state: both indexed in registry
    let entry_a = state
        .repositories
        .registry
        .get_entry(&doc_a_id)
        .await
        .unwrap()
        .expect("file_a.rs should be in registry");
    assert_eq!(entry_a.status, DocumentStatus::Indexed);
    assert_eq!(entry_a.status_reason, None);

    let entry_b = state
        .repositories
        .registry
        .get_entry(&doc_b_id)
        .await
        .unwrap()
        .expect("file_b.rs should be in registry");
    assert_eq!(entry_b.status, DocumentStatus::Indexed);

    // 3. Exclude doc_a via DELETE /api/documents/:id
    let req = Request::builder()
        .uri(format!("/api/documents/{}", doc_a_id.as_uuid()))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let del_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(del_json["success"], true);
    assert_eq!(del_json["id"], doc_a_id.to_string());
    assert_eq!(del_json["status"], "EXCLUDED");
    assert_eq!(
        del_json["message"],
        "Document removed from search index and marked as EXCLUDED."
    );

    // 4. Verify tombstone in registry & removal from search repository
    let excluded_entry_a = state
        .repositories
        .registry
        .get_entry(&doc_a_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(excluded_entry_a.status, DocumentStatus::Excluded);
    assert_eq!(
        excluded_entry_a.status_reason.as_deref(),
        Some("Manually excluded by user")
    );

    // 5. Test idempotency of DELETE /api/documents/:id
    let req_idemp = Request::builder()
        .uri(format!("/api/documents/{}", doc_a_id.as_uuid()))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let res_idemp = app.clone().oneshot(req_idemp).await.unwrap();
    assert_eq!(res_idemp.status(), StatusCode::OK);

    // 6. Test folder rescan DOES NOT resurrect the EXCLUDED document
    let rescan_job_id = orchestrator
        .submit_index_folder(folder.id, true)
        .await
        .unwrap();
    let rescan_cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(rescan_cmd).await.unwrap();

    let rescan_db_job = state
        .repositories
        .job
        .get_job(&rescan_job_id)
        .await
        .unwrap()
        .unwrap();
    // 2 files: doc_b unchanged (skipped), doc_a excluded (skipped) -> files_skipped == 2, files_indexed == 0
    assert_eq!(rescan_db_job.files_indexed, 0);
    assert_eq!(rescan_db_job.files_skipped, 2);

    let entry_a_after_rescan = state
        .repositories
        .registry
        .get_entry(&doc_a_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(entry_a_after_rescan.status, DocumentStatus::Excluded);
    assert_eq!(
        entry_a_after_rescan.status_reason.as_deref(),
        Some("Manually excluded by user")
    );

    // 7. Restore previously EXCLUDED document via POST /api/index
    let restore_payload = json!({
        "folder_id": *folder.id.as_uuid(),
        "relative_path": "file_a.rs"
    });
    let req_restore = Request::builder()
        .uri("/api/index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&restore_payload).unwrap()))
        .unwrap();
    let res_restore = app.clone().oneshot(req_restore).await.unwrap();
    assert_eq!(res_restore.status(), StatusCode::OK);

    let restore_bytes = axum::body::to_bytes(res_restore.into_body(), usize::MAX)
        .await
        .unwrap();
    let restore_json: Value = serde_json::from_slice(&restore_bytes).unwrap();
    assert_eq!(restore_json["document_id"], doc_a_id.to_string());
    assert_eq!(restore_json["status"], "INDEXED");
    assert_eq!(restore_json["message"], "Document indexed successfully.");

    // Verify registry restored to INDEXED
    let restored_entry = state
        .repositories
        .registry
        .get_entry(&doc_a_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(restored_entry.status, DocumentStatus::Indexed);
    assert_eq!(restored_entry.status_reason, None);

    // 8. Next rescan treats restored file as regular INDEXED file
    let _second_rescan_job_id = orchestrator
        .submit_index_folder(folder.id, true)
        .await
        .unwrap();
    let second_rescan_cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(second_rescan_cmd).await.unwrap();

    let entry_a_second_rescan = state
        .repositories
        .registry
        .get_entry(&doc_a_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(entry_a_second_rescan.status, DocumentStatus::Indexed);

    // 9. Dual mode: POST /api/index for brand new file on disk
    let doc_c_path = root.join("file_c.rs");
    fs::write(&doc_c_path, "pub fn func_c() -> i32 { 30 }").unwrap();

    let doc_c_id = DocumentId::from_relative_path(folder.id, "file_c.rs");
    let new_file_payload = json!({
        "folder_id": *folder.id.as_uuid(),
        "relative_path": "file_c.rs"
    });
    let req_new = Request::builder()
        .uri("/api/index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&new_file_payload).unwrap()))
        .unwrap();
    let res_new = app.clone().oneshot(req_new).await.unwrap();
    assert_eq!(res_new.status(), StatusCode::OK);

    let new_entry = state
        .repositories
        .registry
        .get_entry(&doc_c_id)
        .await
        .unwrap()
        .expect("file_c.rs should be indexed");
    assert_eq!(new_entry.status, DocumentStatus::Indexed);
}

#[tokio::test]
async fn test_single_document_exclusion_error_cases() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    // 1. DELETE /api/documents/:id for non-existent document -> 404
    let non_existent_id = uuid::Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/documents/{non_existent_id}"))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let err_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(err_json["code"], "DOCUMENT_NOT_FOUND");

    // 2. POST /api/index with non-existent folder_id -> 404
    let req_folder_not_found = Request::builder()
        .uri("/api/index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "folder_id": non_existent_id,
                "relative_path": "some_file.rs"
            }))
            .unwrap(),
        ))
        .unwrap();
    let res_folder_not_found = app.clone().oneshot(req_folder_not_found).await.unwrap();
    assert_eq!(res_folder_not_found.status(), StatusCode::NOT_FOUND);

    // 3. POST /api/index with path traversal -> 422 validation failure
    let temp = tempdir().unwrap();
    let folder = Folder::new(temp.path().to_path_buf());
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let req_traversal = Request::builder()
        .uri("/api/index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "folder_id": *folder.id.as_uuid(),
                "relative_path": "../escape.rs"
            }))
            .unwrap(),
        ))
        .unwrap();
    let res_traversal = app.clone().oneshot(req_traversal).await.unwrap();
    assert_eq!(res_traversal.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 4. POST /api/index with non-existent file on disk -> 422 validation failure
    let req_missing_file = Request::builder()
        .uri("/api/index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "folder_id": *folder.id.as_uuid(),
                "relative_path": "non_existent.rs"
            }))
            .unwrap(),
        ))
        .unwrap();
    let res_missing_file = app.oneshot(req_missing_file).await.unwrap();
    assert_eq!(res_missing_file.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_get_document_detail_endpoint_success_and_error_cases() {
    let (state, mut rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());
    let orchestrator = state.orchestrator();

    let temp = tempdir().unwrap();
    let root = temp.path();

    let file_path = root.join("hello.rs");
    let file_content = "// Hello Rust LynxSearch\npub fn hello() -> &'static str { \"world\" }";
    fs::write(&file_path, file_content).unwrap();

    let folder = Folder::new(root.to_path_buf());
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let _job_id = orchestrator
        .submit_index_folder(folder.id, false)
        .await
        .unwrap();
    let cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd).await.unwrap();

    let doc_id = DocumentId::from_relative_path(folder.id, "hello.rs");

    // 1. Success GET /api/documents/:id
    let req = Request::builder()
        .uri(format!("/api/documents/{}", doc_id.as_uuid()))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_val: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json_val["id"], doc_id.as_uuid().to_string());
    assert_eq!(json_val["folder_id"], folder.id.as_uuid().to_string());
    assert_eq!(json_val["relative_path"], "hello.rs");
    assert_eq!(json_val["content"], file_content);
    assert_eq!(json_val["type"], "code");
    assert_eq!(json_val["language"], "rust");

    // 2. 404 for unknown document UUID
    let non_existent_id = uuid::Uuid::new_v4();
    let req_404 = Request::builder()
        .uri(format!("/api/documents/{non_existent_id}"))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res_404 = app.clone().oneshot(req_404).await.unwrap();
    assert_eq!(res_404.status(), StatusCode::NOT_FOUND);

    let body_bytes = axum::body::to_bytes(res_404.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_err: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json_err["code"], "DOCUMENT_NOT_FOUND");
}
