use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::dtos::SearchResponseDto;
use backend::domain::models::{DocumentId, DocumentType, FolderId, IndexedDocument, Language};
use backend::state::AppState;
use chrono::Utc;
use tower::ServiceExt;

async fn setup_facets_test_app() -> (axum::Router, AppState, FolderId) {
    let (state, _rx) = AppState::test_state();
    let folder_id = FolderId::new();
    let now = Utc::now();

    let docs = vec![
        IndexedDocument {
            id: DocumentId::from_relative_path(folder_id, "src/auth.rs"),
            folder_id,
            relative_path: "src/auth.rs".to_string(),
            absolute_path: "/workspace/src/auth.rs".to_string(),
            title: "Auth Module".to_string(),
            content: "pub fn authenticate_user(token: &str) -> bool { true }".to_string(),
            tags: vec!["auth".to_string(), "security".to_string()],
            extension: Some("rs".to_string()),
            language: Some(Language::Rust),
            doc_type: DocumentType::Code,
            project: Some("backend".to_string()),
            file_size_bytes: 1024,
            modified_at: now,
            indexed_at: now,
            content_hash: Some("h1".to_string()),
        },
        IndexedDocument {
            id: DocumentId::from_relative_path(folder_id, "src/token.rs"),
            folder_id,
            relative_path: "src/token.rs".to_string(),
            absolute_path: "/workspace/src/token.rs".to_string(),
            title: "Token Parser".to_string(),
            content: "pub fn parse_jwt(token: &str) -> Option<Claims> { None }".to_string(),
            tags: vec!["auth".to_string(), "jwt".to_string()],
            extension: Some("rs".to_string()),
            language: Some(Language::Rust),
            doc_type: DocumentType::Code,
            project: Some("backend".to_string()),
            file_size_bytes: 512,
            modified_at: now,
            indexed_at: now,
            content_hash: Some("h2".to_string()),
        },
        IndexedDocument {
            id: DocumentId::from_relative_path(folder_id, "docs/auth.md"),
            folder_id,
            relative_path: "docs/auth.md".to_string(),
            absolute_path: "/workspace/docs/auth.md".to_string(),
            title: "Authentication Architecture Spec".to_string(),
            content: "System architecture for token authentication and security policies."
                .to_string(),
            tags: vec!["auth".to_string(), "spec".to_string()],
            extension: Some("md".to_string()),
            language: Some(Language::Markdown),
            doc_type: DocumentType::Doc,
            project: Some("docs".to_string()),
            file_size_bytes: 2048,
            modified_at: now,
            indexed_at: now,
            content_hash: Some("h3".to_string()),
        },
        IndexedDocument {
            id: DocumentId::from_relative_path(folder_id, "scripts/deploy.py"),
            folder_id,
            relative_path: "scripts/deploy.py".to_string(),
            absolute_path: "/workspace/scripts/deploy.py".to_string(),
            title: "Deploy Automation".to_string(),
            content: "def deploy():\n    print('deploying infrastructure')".to_string(),
            tags: vec!["infra".to_string(), "deploy".to_string()],
            extension: Some("py".to_string()),
            language: Some(Language::Python),
            doc_type: DocumentType::Code,
            project: Some("infra".to_string()),
            file_size_bytes: 800,
            modified_at: now,
            indexed_at: now,
            content_hash: Some("h4".to_string()),
        },
    ];

    for doc in &docs {
        state.repositories.search.index_document(doc).await.unwrap();
    }

    let router = backend::create_router_with_state(state.clone());
    (router, state, folder_id)
}

#[tokio::test]
async fn test_facet_aggregations_all_categories_returned() {
    let (router, _state, _folder_id) = setup_facets_test_app().await;

    // Search query matching auth documents
    let req = Request::builder()
        .uri("/api/search?q=auth")
        .body(Body::empty())
        .unwrap();

    let res = router.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body.total, 3);

    // 1. Extensions aggregation
    assert!(!body.facets.extensions.is_empty());
    let rs_ext = body
        .facets
        .extensions
        .iter()
        .find(|b| b.key == "rs")
        .unwrap();
    assert_eq!(rs_ext.doc_count, 2);
    let md_ext = body
        .facets
        .extensions
        .iter()
        .find(|b| b.key == "md")
        .unwrap();
    assert_eq!(md_ext.doc_count, 1);

    // 2. Types aggregation
    let code_type = body.facets.types.iter().find(|b| b.key == "code").unwrap();
    assert_eq!(code_type.doc_count, 2);
    let doc_type = body.facets.types.iter().find(|b| b.key == "doc").unwrap();
    assert_eq!(doc_type.doc_count, 1);

    // 3. Languages aggregation
    let rust_lang = body
        .facets
        .languages
        .iter()
        .find(|b| b.key == "rust")
        .unwrap();
    assert_eq!(rust_lang.doc_count, 2);
    let md_lang = body
        .facets
        .languages
        .iter()
        .find(|b| b.key == "markdown")
        .unwrap();
    assert_eq!(md_lang.doc_count, 1);

    // 4. Projects aggregation
    let backend_proj = body
        .facets
        .projects
        .iter()
        .find(|b| b.key == "backend")
        .unwrap();
    assert_eq!(backend_proj.doc_count, 2);
    let docs_proj = body
        .facets
        .projects
        .iter()
        .find(|b| b.key == "docs")
        .unwrap();
    assert_eq!(docs_proj.doc_count, 1);

    // 5. Tags aggregation
    let auth_tag = body.facets.tags.iter().find(|b| b.key == "auth").unwrap();
    assert_eq!(auth_tag.doc_count, 3);
}

#[tokio::test]
async fn test_facets_reflect_active_query_and_filters() {
    let (router, _state, _folder_id) = setup_facets_test_app().await;

    // Filter by type:doc -> hits filtered to 1 doc, but facets retain all query options for multi-select
    let req = Request::builder()
        .uri("/api/search?q=auth&type=doc")
        .body(Body::empty())
        .unwrap();

    let res = router.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();

    // Hits are post-filtered to only the doc
    assert_eq!(body.total, 1);
    assert_eq!(body.items[0].relative_path, "docs/auth.md");

    // Facets retain full query scope (code + doc) so users can multi-select
    assert_eq!(body.facets.types.len(), 2);
    let doc_type = body.facets.types.iter().find(|b| b.key == "doc").unwrap();
    assert_eq!(doc_type.doc_count, 1);
    let code_type = body.facets.types.iter().find(|b| b.key == "code").unwrap();
    assert_eq!(code_type.doc_count, 2);

    let md_ext = body
        .facets
        .extensions
        .iter()
        .find(|b| b.key == "md")
        .unwrap();
    assert_eq!(md_ext.doc_count, 1);
    let rs_ext = body
        .facets
        .extensions
        .iter()
        .find(|b| b.key == "rs")
        .unwrap();
    assert_eq!(rs_ext.doc_count, 2);
}

#[tokio::test]
async fn test_facets_with_multi_select_values() {
    let (router, _state, _folder_id) = setup_facets_test_app().await;

    // Multi-select extensions: rs,py
    let req = Request::builder()
        .uri("/api/search?extension=rs,py")
        .body(Body::empty())
        .unwrap();

    let res = router.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();

    // 2 rs docs + 1 py doc = 3 docs
    assert_eq!(body.total, 3);
    let rs_ext = body
        .facets
        .extensions
        .iter()
        .find(|b| b.key == "rs")
        .unwrap();
    assert_eq!(rs_ext.doc_count, 2);
    let py_ext = body
        .facets
        .extensions
        .iter()
        .find(|b| b.key == "py")
        .unwrap();
    assert_eq!(py_ext.doc_count, 1);
}

#[tokio::test]
async fn test_empty_query_handles_empty_or_zero_facets_cleanly() {
    let (router, _state, _folder_id) = setup_facets_test_app().await;

    // Query non-existent keyword
    let req = Request::builder()
        .uri("/api/search?q=non_existent_symbol_xyz")
        .body(Body::empty())
        .unwrap();

    let res = router.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body.total, 0);
    assert!(body.facets.extensions.is_empty());
    assert!(body.facets.types.is_empty());
    assert!(body.facets.languages.is_empty());
    assert!(body.facets.tags.is_empty());
    assert!(body.facets.projects.is_empty());
}
