use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::dtos::{SearchResponseDto, SuggestResponseDto};
use backend::config::DEFAULT_ELASTICSEARCH_URL;
use backend::domain::models::{
    AppSettings, DocumentId, DocumentType, FolderId, IndexedDocument, Language,
};
use backend::domain::ports::SearchRepository;
use backend::infrastructure::elasticsearch::{
    DEFAULT_PING_TIMEOUT, EsSearchRepository, create_es_client_with_timeout,
    document_index_schema_with_alias, ping_elasticsearch_with_timeout,
};
use backend::state::{AppState, Repositories};
use chrono::{Duration as ChronoDuration, Utc};
use elasticsearch::indices::{IndicesCreateParts, IndicesDeleteParts, IndicesRefreshParts};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

/// Helper untuk membuat dokumen uji terpadu Phase 7 Gate.
fn create_phase7_test_documents(folder_id: FolderId) -> Vec<IndexedDocument> {
    let now = Utc::now();

    // Doc 1: Markdown doc tentang Ownership (Markdown, Type: Doc, Project: core)
    let doc1 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/ownership_guide.md"),
        folder_id,
        relative_path: "docs/ownership_guide.md".to_string(),
        absolute_path: "/workspace/docs/ownership_guide.md".to_string(),
        title: "Rust Ownership Guide".to_string(),
        content: "Rust memory management without GC.\nOwnership rules and borrow checker ensure safety.\nEvery value in Rust has an owner."
            .to_string(),
        tags: vec!["rust".to_string(), "guide".to_string()],
        extension: Some("md".to_string()),
        language: Some(Language::Markdown),
        doc_type: DocumentType::Doc,
        project: Some("core".to_string()),
        file_size_bytes: 1024,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash1".to_string()),
    };

    // Doc 2: Rust code tentang Auth (Rust, Type: Code, Project: backend)
    let doc2 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/auth/service.rs"),
        folder_id,
        relative_path: "src/auth/service.rs".to_string(),
        absolute_path: "/workspace/src/auth/service.rs".to_string(),
        title: "Authentication Service".to_string(),
        content: "pub fn authenticate_user(token: &str) -> bool { true }\nValidates bearer tokens and credentials."
            .to_string(),
        tags: vec!["auth".to_string(), "security".to_string()],
        extension: Some("rs".to_string()),
        language: Some(Language::Rust),
        doc_type: DocumentType::Code,
        project: Some("backend".to_string()),
        file_size_bytes: 2048,
        modified_at: now - ChronoDuration::days(1),
        indexed_at: now,
        content_hash: Some("hash2".to_string()),
    };

    // Doc 3: Rust code tentang Engine Worker (Rust, Type: Code, Project: backend)
    let doc3 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/engine/worker.rs"),
        folder_id,
        relative_path: "src/engine/worker.rs".to_string(),
        absolute_path: "/workspace/src/engine/worker.rs".to_string(),
        title: "Engine Worker Queue".to_string(),
        content: "pub fn run_worker_pool() {}\nThread worker pool manages async jobs and transfers ownership across channels."
            .to_string(),
        tags: vec!["engine".to_string(), "concurrency".to_string()],
        extension: Some("rs".to_string()),
        language: Some(Language::Rust),
        doc_type: DocumentType::Code,
        project: Some("backend".to_string()),
        file_size_bytes: 4096,
        modified_at: now - ChronoDuration::days(2),
        indexed_at: now,
        content_hash: Some("hash3".to_string()),
    };

    // Doc 4: Markdown doc berisi typo intentional "ownrship" (Markdown, Type: Doc, Project: notes)
    let doc4 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/typo_note.md"),
        folder_id,
        relative_path: "docs/typo_note.md".to_string(),
        absolute_path: "/workspace/docs/typo_note.md".to_string(),
        title: "Common Typo in Rust Ownrship".to_string(),
        content: "Developers frequently misspell ownership as ownrship in code comments."
            .to_string(),
        tags: vec![
            "rust".to_string(),
            "notes".to_string(),
            "ownrship".to_string(),
        ],
        extension: Some("md".to_string()),
        language: Some(Language::Markdown),
        doc_type: DocumentType::Doc,
        project: Some("notes".to_string()),
        file_size_bytes: 512,
        modified_at: now - ChronoDuration::days(3),
        indexed_at: now,
        content_hash: Some("hash4".to_string()),
    };

    // Doc 5: Yaml config (Type: Config, Project: core)
    let doc5 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "config/app.yaml"),
        folder_id,
        relative_path: "config/app.yaml".to_string(),
        absolute_path: "/workspace/config/app.yaml".to_string(),
        title: "Application Config".to_string(),
        content: "server:\n  port: 3001\n  host: 127.0.0.1\n  workers: 4".to_string(),
        tags: vec!["config".to_string(), "deployment".to_string()],
        extension: Some("yaml".to_string()),
        language: None,
        doc_type: DocumentType::Config,
        project: Some("core".to_string()),
        file_size_bytes: 256,
        modified_at: now - ChronoDuration::days(4),
        indexed_at: now,
        content_hash: Some("hash5".to_string()),
    };

    vec![doc1, doc2, doc3, doc4, doc5]
}

/// Verifikasi seluruh kriteria Phase 7 Gate (Task 8.1 - 8.14)
async fn verify_phase7_gate_assertions(app: axum::Router, state: AppState) {
    // ------------------------------------------------------------------------
    // 1. Parser & Warning Suite
    // ------------------------------------------------------------------------
    let req = Request::builder()
        .uri("/api/search?q=ownership%20unknown:custom%20tag:")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert!(
        body.warnings.iter().any(|w| w.contains("unknown")),
        "Warning untuk unknown filter harus muncul: {:?}",
        body.warnings
    );
    assert!(
        body.warnings.iter().any(|w| w.contains("nilai kosong")),
        "Warning untuk empty filter harus muncul: {:?}",
        body.warnings
    );
    assert!(
        body.total >= 1,
        "Pencarian kata kunci 'ownership' tetap harus berhasil meski ada warning"
    );

    // ------------------------------------------------------------------------
    // 2. Inline Filters Suite
    // ------------------------------------------------------------------------
    // a. type:code & language:rust
    let req = Request::builder()
        .uri("/api/search?q=type:code%20language:rust")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.items.len(), 2);
    for item in &body.items {
        assert_eq!(item.doc_type, DocumentType::Code);
        assert_eq!(item.language, Some(Language::Rust));
    }

    // b. tag:security
    let req = Request::builder()
        .uri("/api/search?q=tag:security")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.items.len(), 1);
    assert_eq!(body.items[0].relative_path, "src/auth/service.rs");

    // c. project:backend
    let req = Request::builder()
        .uri("/api/search?q=project:backend")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.items.len(), 2);

    // d. ext:yaml
    let req = Request::builder()
        .uri("/api/search?q=ext:yaml")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.items.len(), 1);
    assert_eq!(body.items[0].relative_path, "config/app.yaml");

    // ------------------------------------------------------------------------
    // 3. Facet Aggregations Suite & Post-Filter Sync
    // ------------------------------------------------------------------------
    let req = Request::builder()
        .uri("/api/search?q=ownership")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert!(
        !body.facets.types.is_empty(),
        "Facet types harus tersedia pada hasil pencarian"
    );
    assert!(
        !body.facets.extensions.is_empty(),
        "Facet extensions harus tersedia"
    );

    // Filter dengan post_filter: type:doc
    // Dokumen yang dikembalikan hanya type doc, tetapi agregasi facets tetap komprehensif
    let req = Request::builder()
        .uri("/api/search?q=ownership%20type:doc")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    for item in &body.items {
        assert_eq!(item.doc_type, DocumentType::Doc);
    }

    // ------------------------------------------------------------------------
    // 4. Fuzzy Integration Suite
    // ------------------------------------------------------------------------
    // Pencarian dengan typo 'ownrship' harus mencocokkan 'ownership'
    let req = Request::builder()
        .uri("/api/search?q=ownrship")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert!(
        body.items
            .iter()
            .any(|i| i.relative_path == "docs/ownership_guide.md"
                || i.relative_path == "docs/typo_note.md"),
        "Fuzzy search 'ownrship' harus menemukan dokumen terkait ownership"
    );

    // ------------------------------------------------------------------------
    // 5. Prefix Integration Suite
    // ------------------------------------------------------------------------
    let req = Request::builder()
        .uri("/api/search?q=owner")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert!(
        body.items
            .iter()
            .any(|i| i.relative_path == "docs/ownership_guide.md"),
        "Prefix search 'owner' harus menemukan 'Rust Ownership Guide'"
    );

    // ------------------------------------------------------------------------
    // 6. Autocomplete Integration Suite (/api/suggest)
    // ------------------------------------------------------------------------
    let req = Request::builder()
        .uri("/api/suggest?q=own")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SuggestResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert!(
        !body.suggestions.is_empty(),
        "Suggest untuk prefix 'own' harus mengembalikan saran"
    );

    let req = Request::builder()
        .uri("/api/suggest?q=auth")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SuggestResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert!(
        !body.suggestions.is_empty(),
        "Suggest untuk prefix 'auth' harus mengembalikan saran"
    );

    // ------------------------------------------------------------------------
    // 7. Sort Suite
    // ------------------------------------------------------------------------
    // a. Sort name_asc
    let req = Request::builder()
        .uri("/api/search?q=type:code&sort=name_asc")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.items.len(), 2);
    assert_eq!(body.items[0].relative_path, "src/auth/service.rs");
    assert_eq!(body.items[1].relative_path, "src/engine/worker.rs");

    // b. Sort modified_desc
    let req = Request::builder()
        .uri("/api/search?q=type:code&sort=modified_desc")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.items.len(), 2);
    // auth/service.rs modified_at 1 hari lalu vs worker.rs 2 hari lalu -> auth/service.rs duluan
    assert_eq!(body.items[0].relative_path, "src/auth/service.rs");

    // ------------------------------------------------------------------------
    // 8. Pagination + Sort Suite
    // ------------------------------------------------------------------------
    let req_p1 = Request::builder()
        .uri("/api/search?q=type:code&sort=name_asc&page=1&size=1")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_p1 = app.clone().oneshot(req_p1).await.unwrap();
    assert_eq!(res_p1.status(), StatusCode::OK);
    let bytes_p1 = axum::body::to_bytes(res_p1.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_p1: SearchResponseDto = serde_json::from_slice(&bytes_p1).unwrap();
    assert_eq!(body_p1.items.len(), 1);
    assert_eq!(body_p1.items[0].relative_path, "src/auth/service.rs");

    let req_p2 = Request::builder()
        .uri("/api/search?q=type:code&sort=name_asc&page=2&size=1")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_p2 = app.clone().oneshot(req_p2).await.unwrap();
    assert_eq!(res_p2.status(), StatusCode::OK);
    let bytes_p2 = axum::body::to_bytes(res_p2.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_p2: SearchResponseDto = serde_json::from_slice(&bytes_p2).unwrap();
    assert_eq!(body_p2.items.len(), 1);
    assert_eq!(body_p2.items[0].relative_path, "src/engine/worker.rs");

    // ------------------------------------------------------------------------
    // 9. BM25 Tuning Suite
    // ------------------------------------------------------------------------
    // Verifikasi initial settings
    let req = Request::builder()
        .uri("/api/settings")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let initial_settings: AppSettings = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(initial_settings.weights.title, 3.0);

    // Update weights via PUT /api/settings
    let update_body = json!({
        "weights": {
            "title": 7.0,
            "tags": 4.0,
            "content": 0.5
        }
    });
    let req = Request::builder()
        .uri("/api/settings")
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let updated_settings = state.get_settings().await;
    assert_eq!(updated_settings.weights.title, 7.0);
    assert_eq!(updated_settings.weights.tags, 4.0);
    assert_eq!(updated_settings.weights.content, 0.5);

    // Reset settings via POST /api/settings/reset
    let req = Request::builder()
        .uri("/api/settings/reset")
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let reset_settings = state.get_settings().await;
    assert_eq!(reset_settings.weights.title, 3.0);
    assert_eq!(reset_settings.weights.tags, 2.0);
    assert_eq!(reset_settings.weights.content, 1.0);
}

#[tokio::test]
async fn test_phase_7_gate_in_memory() {
    let (state, _rx) = AppState::test_state();
    let folder_id = FolderId::new();
    let docs = create_phase7_test_documents(folder_id);

    for doc in &docs {
        state
            .repositories
            .search
            .index_document(doc)
            .await
            .expect("Failed to index test document in-memory");
    }

    let app = backend::create_router_with_state(state.clone());
    verify_phase7_gate_assertions(app, state).await;
}

#[tokio::test]
async fn test_phase_7_gate_live_elasticsearch() {
    let _ = dotenvy::dotenv();
    let es_url = std::env::var("ELASTICSEARCH_URL")
        .unwrap_or_else(|_| DEFAULT_ELASTICSEARCH_URL.to_string());

    let client = match create_es_client_with_timeout(&es_url, Duration::from_secs(10)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping live ES test: cannot connect at {es_url}: {e}");
            return;
        }
    };

    if ping_elasticsearch_with_timeout(&client, DEFAULT_PING_TIMEOUT)
        .await
        .is_err()
    {
        eprintln!("Skipping live ES test: Elasticsearch not reachable at {es_url}");
        return;
    }

    let test_suffix = Uuid::new_v4().simple().to_string();
    let test_alias = format!("lynx_phase7_alias_{test_suffix}");
    let test_index = format!("lynx_phase7_index_{test_suffix}");

    // 1. Buat index dan alias pada live Elasticsearch
    let schema = document_index_schema_with_alias(&test_alias);
    let create_res = client
        .indices()
        .create(IndicesCreateParts::Index(&test_index))
        .body(schema)
        .send()
        .await
        .expect("Create test index failed");
    assert!(create_res.status_code().is_success());

    // 2. Siapkan repository & AppState yang menggunakan EsSearchRepository
    let es_repo = Arc::new(EsSearchRepository::new(client.clone(), test_alias.clone()));
    let folder_id = FolderId::new();
    let docs = create_phase7_test_documents(folder_id);

    for doc in &docs {
        es_repo
            .index_document(doc)
            .await
            .expect("Failed to index test document in live ES");
    }

    // Refresh index live agar dokumen langsung searchable
    let _ = client
        .indices()
        .refresh(IndicesRefreshParts::Index(&[&test_index]))
        .send()
        .await;

    let (base_state, _rx) = AppState::test_state();
    let mut repos = Repositories::in_memory();
    repos.search = es_repo;
    let state = AppState::new(
        base_state.db_pool.clone(),
        base_state.es_client.clone(),
        repos,
        base_state.job_tracker.clone(),
        base_state.worker_sender.clone(),
        base_state.file_io_semaphore.clone(),
        base_state.config.clone(),
        base_state.settings.clone(),
    );

    let app = backend::create_router_with_state(state.clone());

    // 3. Verifikasi seluruh assertions Gate Phase 7 pada live Elasticsearch
    verify_phase7_gate_assertions(app, state).await;

    // 4. Bersihkan index uji
    let _ = client
        .indices()
        .delete(IndicesDeleteParts::Index(&[&test_index]))
        .send()
        .await;
}
