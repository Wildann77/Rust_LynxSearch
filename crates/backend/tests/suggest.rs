use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::dtos::SuggestResponseDto;
use backend::domain::models::types::DocumentType;
use backend::domain::models::{DocumentId, FolderId, IndexedDocument};
use backend::domain::ports::SearchRepository;
use chrono::Utc;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_suggest_endpoint_returns_matching_suggestions() {
    let (state, _rx) = backend::AppState::test_state();
    let folder_id = FolderId::new();
    let now = Utc::now();

    // Seed dummy documents into in-memory search repository
    let doc1 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/auth/service.rs"),
        folder_id,
        relative_path: "src/auth/service.rs".to_string(),
        absolute_path: "/workspace/src/auth/service.rs".to_string(),
        title: "Authentication Service".to_string(),
        content: "Handles user login and credentials.".to_string(),
        tags: vec!["auth".to_string()],
        extension: Some("rs".to_string()),
        language: None,
        doc_type: DocumentType::Code,
        project: None,
        file_size_bytes: 1024,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash1".to_string()),
    };

    let doc2 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/auth/middleware.rs"),
        folder_id,
        relative_path: "src/auth/middleware.rs".to_string(),
        absolute_path: "/workspace/src/auth/middleware.rs".to_string(),
        title: "Authentication Middleware".to_string(),
        content: "Validates bearer token.".to_string(),
        tags: vec!["auth".to_string()],
        extension: Some("rs".to_string()),
        language: None,
        doc_type: DocumentType::Code,
        project: None,
        file_size_bytes: 512,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash2".to_string()),
    };

    let doc3 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/user/controller.rs"),
        folder_id,
        relative_path: "src/user/controller.rs".to_string(),
        absolute_path: "/workspace/src/user/controller.rs".to_string(),
        title: "User Controller".to_string(),
        content: "API handlers for user management.".to_string(),
        tags: vec!["user".to_string()],
        extension: Some("rs".to_string()),
        language: None,
        doc_type: DocumentType::Code,
        project: None,
        file_size_bytes: 2048,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash3".to_string()),
    };

    state
        .repositories
        .search
        .index_document(&doc1)
        .await
        .unwrap();
    state
        .repositories
        .search
        .index_document(&doc2)
        .await
        .unwrap();
    state
        .repositories
        .search
        .index_document(&doc3)
        .await
        .unwrap();

    let app = backend::create_router_with_state(state.clone());

    // 1. Query matching "Auth"
    let req = Request::builder()
        .uri("/api/suggest?q=auth")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 1024 * 16)
        .await
        .unwrap();
    let dto: SuggestResponseDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(dto.suggestions.len(), 2);
    assert!(
        dto.suggestions
            .contains(&"Authentication Service".to_string())
    );
    assert!(
        dto.suggestions
            .contains(&"Authentication Middleware".to_string())
    );

    // 2. Query with limit=1
    let req = Request::builder()
        .uri("/api/suggest?q=auth&limit=1")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 1024 * 16)
        .await
        .unwrap();
    let dto: SuggestResponseDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(dto.suggestions.len(), 1);

    // 3. Query without any match
    let req = Request::builder()
        .uri("/api/suggest?q=nonexistent")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 1024 * 16)
        .await
        .unwrap();
    let dto: SuggestResponseDto = serde_json::from_slice(&body).unwrap();
    assert!(dto.suggestions.is_empty());
}

#[tokio::test]
async fn test_suggest_validation_rules() {
    let (state, _rx) = backend::AppState::test_state();
    let app = backend::create_router_with_state(state);

    // Missing query parameter "q" -> 422
    let req = Request::builder()
        .uri("/api/suggest")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Empty query parameter "q" -> 422 (min = 1)
    let req = Request::builder()
        .uri("/api/suggest?q=")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Limit = 0 out of bounds -> 422
    let req = Request::builder()
        .uri("/api/suggest?q=test&limit=0")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Limit = 51 out of bounds -> 422
    let req = Request::builder()
        .uri("/api/suggest?q=test&limit=51")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// Stub search repository that always returns error to verify graceful degradation
struct FailingSearchRepository;

#[async_trait::async_trait]
impl SearchRepository for FailingSearchRepository {
    fn search_alias(&self) -> &str {
        "lynx_documents"
    }
    async fn index_document(&self, _doc: &IndexedDocument) -> Result<(), backend::error::AppError> {
        Ok(())
    }
    async fn bulk_index_documents(
        &self,
        _docs: &[IndexedDocument],
    ) -> Result<backend::domain::models::BulkIndexReport, backend::error::AppError> {
        Ok(backend::domain::models::BulkIndexReport::default())
    }
    async fn bulk_index_to_target(
        &self,
        _target: &str,
        _docs: &[IndexedDocument],
    ) -> Result<backend::domain::models::BulkIndexReport, backend::error::AppError> {
        Ok(backend::domain::models::BulkIndexReport::default())
    }
    async fn delete_document(
        &self,
        _id: &backend::domain::models::DocumentId,
    ) -> Result<(), backend::error::AppError> {
        Ok(())
    }
    async fn delete_documents_by_folder(
        &self,
        _folder_id: &backend::domain::models::FolderId,
    ) -> Result<u64, backend::error::AppError> {
        Ok(0)
    }
    async fn search(
        &self,
        _query_dsl: &serde_json::value::RawValue,
    ) -> Result<backend::domain::models::SearchRawResponse, backend::error::AppError> {
        Err(backend::error::AppError::SearchEngine(
            "ES is down".to_string(),
        ))
    }
    async fn suggest(
        &self,
        _prefix: &str,
        _limit: usize,
    ) -> Result<Vec<String>, backend::error::AppError> {
        Err(backend::error::AppError::SearchEngine(
            "ES connection refused".to_string(),
        ))
    }
    async fn rebuild_index_with_alias(
        &self,
        _new_index: &str,
        _alias: &str,
    ) -> Result<(), backend::error::AppError> {
        Ok(())
    }
    async fn ping(&self) -> Result<(), backend::error::AppError> {
        Err(backend::error::AppError::SearchEngine(
            "ES down".to_string(),
        ))
    }
    async fn ensure_initial_index(&self) -> Result<String, backend::error::AppError> {
        Ok("test".to_string())
    }
    async fn get_active_physical_index(&self) -> Result<Option<String>, backend::error::AppError> {
        Ok(Some("test".to_string()))
    }
    async fn create_versioned_index(
        &self,
        _version: u32,
    ) -> Result<String, backend::error::AppError> {
        Ok("test".to_string())
    }
    async fn get_alias_indices(
        &self,
        _alias: &str,
    ) -> Result<Vec<String>, backend::error::AppError> {
        Ok(vec![])
    }
    async fn swap_alias(
        &self,
        _alias: &str,
        _old: &[String],
        _new: &str,
    ) -> Result<(), backend::error::AppError> {
        Ok(())
    }
    async fn delete_index(&self, _index_name: &str) -> Result<(), backend::error::AppError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_suggest_graceful_degradation_when_search_engine_fails() {
    let (base_state, _rx) = backend::AppState::test_state();
    let mut repos = backend::Repositories::in_memory();
    repos.search = Arc::new(FailingSearchRepository);
    let state = backend::AppState::new(
        base_state.db_pool.clone(),
        base_state.es_client.clone(),
        repos,
        base_state.job_tracker.clone(),
        base_state.worker_sender.clone(),
        base_state.file_io_semaphore.clone(),
        base_state.config.clone(),
        base_state.settings.clone(),
    );

    let app = backend::create_router_with_state(state);

    let req = Request::builder()
        .uri("/api/suggest?q=elasticsearch_down")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    // Must return 200 OK with empty suggestions instead of failing with 500
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 1024 * 16)
        .await
        .unwrap();
    let dto: SuggestResponseDto = serde_json::from_slice(&body).unwrap();
    assert!(dto.suggestions.is_empty());
}
