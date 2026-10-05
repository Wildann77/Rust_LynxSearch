use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use backend::api::dtos::{DeleteFolderResponseDto, FolderResponseDto, IndexFolderResponseDto};
use backend::create_router_with_state;
use backend::domain::models::{DocumentId, Folder, FolderId, FolderStatus, IndexedDocument};
use backend::error::{ErrorCode, ErrorResponse};
use backend::state::AppState;
use chrono::Utc;
use serde_json::json;
use std::path::PathBuf;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_get_folders_empty_and_populated() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    // 1. Initial empty state
    let req = Request::builder()
        .uri("/api/folders")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let folders: Vec<FolderResponseDto> = serde_json::from_slice(&bytes).unwrap();
    assert!(folders.is_empty());

    // 2. Add folders with custom document count
    let folder_1 = Folder::new(PathBuf::from("/workspace/project_alpha"));
    let folder_2 = Folder::new(PathBuf::from("/workspace/project_beta"));

    state
        .repositories
        .folder
        .create_folder(&folder_1)
        .await
        .unwrap();
    state
        .repositories
        .folder
        .create_folder(&folder_2)
        .await
        .unwrap();

    // Set document counts in in-memory test repo
    if let Some(in_mem) = state
        .repositories
        .folder
        .as_any()
        .downcast_ref::<backend::domain::ports::stubs::InMemoryFolderRepository>()
    {
        in_mem.set_document_count(folder_1.id, 42);
        in_mem.set_document_count(folder_2.id, 108);
    }

    let req = Request::builder()
        .uri("/api/folders")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let folders: Vec<FolderResponseDto> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(folders.len(), 2);

    assert_eq!(folders[0].id, *folder_1.id.as_uuid());
    assert_eq!(folders[0].root_path, "/workspace/project_alpha");
    assert_eq!(folders[0].status, FolderStatus::Idle);
    assert_eq!(folders[0].document_count, 42);

    assert_eq!(folders[1].id, *folder_2.id.as_uuid());
    assert_eq!(folders[1].root_path, "/workspace/project_beta");
    assert_eq!(folders[1].status, FolderStatus::Idle);
    assert_eq!(folders[1].document_count, 108);
}

#[tokio::test]
async fn test_post_index_folder_new_folder_starts_import() {
    let (state, mut rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let payload = json!({
        "root_path": "/home/developer/projects/fresh_app"
    });

    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::ACCEPTED);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let resp: IndexFolderResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(resp.status, "RUNNING");

    // Verify folder was persisted
    let folder_id = FolderId::from_uuid(resp.folder_id);
    let folder_opt = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap();
    assert!(folder_opt.is_some());
    let folder = folder_opt.unwrap();
    assert_eq!(
        folder.path,
        PathBuf::from("/home/developer/projects/fresh_app")
    );

    // Verify command was sent to worker channel
    let cmd = rx.recv().await.expect("Worker command must be enqueued");
    match cmd {
        backend::application::orchestrator::WorkerCommand::IndexFolder {
            job_id,
            folder_id: cmd_fid,
            rescan,
        } => {
            assert_eq!(*job_id.as_uuid(), resp.job_id);
            assert_eq!(cmd_fid, folder_id);
            assert!(
                !rescan,
                "New folder must trigger initial import (rescan = false)"
            );
        }
        other => panic!("Unexpected worker command: {other:?}"),
    }
}

#[tokio::test]
async fn test_post_index_folder_duplicate_root_conflict() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let path_str = "/home/developer/projects/duplicate_test";
    let payload = json!({ "root_path": path_str });

    // 1. First registration succeeds
    let req1 = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::ACCEPTED);
    let bytes1 = to_bytes(res1.into_body(), usize::MAX).await.unwrap();
    let resp1: IndexFolderResponseDto = serde_json::from_slice(&bytes1).unwrap();

    // 2. Second registration with exact same path must fail with 409 Conflict
    let req2 = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::CONFLICT);

    let bytes2 = to_bytes(res2.into_body(), usize::MAX).await.unwrap();
    let err_resp: ErrorResponse = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(err_resp.code, ErrorCode::JobConflict);
    assert!(err_resp.message.contains("already registered"));

    // Verify error details identify the conflicting folder
    let details = err_resp.details.expect("Conflict details must be present");
    assert_eq!(details["root_path"], path_str);
    assert_eq!(details["folder_id"], resp1.folder_id.to_string());
}

#[tokio::test]
async fn test_post_index_folder_existing_id_triggers_rescan() {
    let (state, mut rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let folder = Folder::new(PathBuf::from("/workspace/rescan_target"));
    let folder_id = folder.id;
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let payload = json!({
        "folder_id": *folder_id.as_uuid()
    });

    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::ACCEPTED);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let resp: IndexFolderResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(resp.folder_id, *folder_id.as_uuid());

    // Verify command was sent with rescan = true
    let cmd = rx.recv().await.expect("Worker command must be enqueued");
    match cmd {
        backend::application::orchestrator::WorkerCommand::IndexFolder {
            folder_id: cmd_fid,
            rescan,
            ..
        } => {
            assert_eq!(cmd_fid, folder_id);
            assert!(
                rescan,
                "Existing folder_id must trigger rescan (rescan = true)"
            );
        }
        other => panic!("Unexpected worker command: {other:?}"),
    }
}

#[tokio::test]
async fn test_post_index_folder_nonexistent_id_returns_404() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state);

    let random_id = Uuid::new_v4();
    let payload = json!({
        "folder_id": random_id
    });

    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_resp: ErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(err_resp.code, ErrorCode::FolderNotFound);
}

#[tokio::test]
async fn test_post_index_folder_validation_errors() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state);

    // 1. Missing both folder_id and root_path
    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 2. Path traversal
    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"root_path":"/home/../etc/passwd"}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 3. Relative path
    let req = Request::builder()
        .uri("/api/index/folder")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"root_path":"relative/dir"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_delete_folder_success() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let folder = Folder::new(PathBuf::from("/workspace/delete_target"));
    let folder_id = folder.id;
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // Index a mock document under this folder in search repository
    let doc = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/main.rs"),
        folder_id,
        relative_path: "src/main.rs".to_string(),
        absolute_path: "/workspace/delete_target/src/main.rs".to_string(),
        title: "Main".to_string(),
        content: "fn main() {}".to_string(),
        tags: vec![],
        extension: Some("rs".to_string()),
        language: Some(backend::domain::models::Language::Rust),
        doc_type: backend::domain::models::DocumentType::Code,
        project: None,
        file_size_bytes: 12,
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
        content_hash: Some("hash123".to_string()),
    };
    state
        .repositories
        .search
        .index_document(&doc)
        .await
        .unwrap();

    // Send DELETE /api/folders/:id
    let req = Request::builder()
        .uri(format!("/api/folders/{folder_id}"))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let resp: DeleteFolderResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert!(resp.success);
    assert_eq!(resp.folder_id, *folder_id.as_uuid());
    assert_eq!(resp.deleted_documents, 1);

    // Folder must no longer exist in repository
    let exists = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap();
    assert!(exists.is_none());
}

#[tokio::test]
async fn test_delete_folder_nonexistent_returns_404() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state);

    let random_id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/folders/{random_id}"))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_resp: ErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(err_resp.code, ErrorCode::FolderNotFound);
}

#[tokio::test]
async fn test_delete_folder_while_scanning_returns_409() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    let folder = Folder::new(PathBuf::from("/workspace/scanning_target"));
    let folder_id = folder.id;
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // Lock folder to simulate active scan
    state.job_tracker.try_lock_folder(&folder_id).unwrap();

    let req = Request::builder()
        .uri(format!("/api/folders/{folder_id}"))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_resp: ErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(err_resp.code, ErrorCode::JobConflict);
}
