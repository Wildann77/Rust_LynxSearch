use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::dtos::SearchResponseDto;
use backend::config::DEFAULT_ELASTICSEARCH_URL;
use backend::domain::models::{DocumentId, DocumentType, FolderId, IndexedDocument, Language};
use backend::domain::ports::SearchRepository;
use backend::infrastructure::elasticsearch::{
    DEFAULT_PING_TIMEOUT, EsSearchRepository, create_es_client_with_timeout,
    document_index_schema_with_alias, ping_elasticsearch_with_timeout,
};
use backend::state::{AppState, Repositories};
use chrono::Utc;
use elasticsearch::indices::{IndicesCreateParts, IndicesDeleteParts, IndicesRefreshParts};
use tower::ServiceExt;
use uuid::Uuid;

/// Helper untuk membuat dokumen uji Phase 5 Gate.
fn create_test_documents(
    folder_id: FolderId,
) -> (IndexedDocument, IndexedDocument, IndexedDocument) {
    let now = Utc::now();

    // Doc 1: "Rust Ownership" di Title dan Content -> Harus rank #1 karena Title Boost (3.0 vs 1.0)
    let doc1 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/ownership_guide.md"),
        folder_id,
        relative_path: "docs/ownership_guide.md".to_string(),
        absolute_path: "/workspace/docs/ownership_guide.md".to_string(),
        title: "Rust Ownership Guide".to_string(),
        content: "Detailed documentation about Rust memory management.\nOwnership ensures memory safety without GC.\nBorrow checker validates references."
            .to_string(),
        tags: vec!["rust".to_string(), "guide".to_string()],
        extension: Some("md".to_string()),
        language: None,
        doc_type: DocumentType::Doc,
        project: Some("core".to_string()),
        file_size_bytes: 1024,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash1".to_string()),
    };

    // Doc 2: "ownership" hanya di Content (Title tidak memiliki keyword) -> Harus rank #2
    let doc2 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/memory_internals.md"),
        folder_id,
        relative_path: "docs/memory_internals.md".to_string(),
        absolute_path: "/workspace/docs/memory_internals.md".to_string(),
        title: "Memory Internals Deep Dive".to_string(),
        content: "Understanding how ownership works behind the scenes in system programming.\nStack and heap allocations explained."
            .to_string(),
        tags: vec!["memory".to_string()],
        extension: Some("md".to_string()),
        language: None,
        doc_type: DocumentType::Doc,
        project: Some("core".to_string()),
        file_size_bytes: 2048,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash2".to_string()),
    };

    // Doc 3: Kode sumber dengan multiple fragments pada baris terpisah
    let doc3 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/engine.rs"),
        folder_id,
        relative_path: "src/engine.rs".to_string(),
        absolute_path: "/workspace/src/engine.rs".to_string(),
        title: "Engine Implementation".to_string(),
        content: "line 1: fn init() {}\nline 2: fn transfer_ownership(res: Resource) {}\nline 3: // internal processing\nline 4: fn drop_ownership(res: Resource) {}\nline 5: // end of file"
            .to_string(),
        tags: vec!["code".to_string()],
        extension: Some("rs".to_string()),
        language: Some(Language::Rust),
        doc_type: DocumentType::Code,
        project: Some("engine".to_string()),
        file_size_bytes: 512,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash3".to_string()),
    };

    (doc1, doc2, doc3)
}

/// Verifikasi Phase 5 Gate: BM25 boost, scores, latency, highlight, line number, pagination
async fn verify_search_gate_assertions(app: axum::Router) {
    // 1. /api/search returns 200 OK, score, and latency
    let req = Request::builder()
        .uri("/api/search?q=ownership")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body.total, 3, "Total dokumen yang cocok harus 3");
    assert_eq!(body.items.len(), 3);
    assert_eq!(body.results.len(), 3);
    let _ = body.took_ms;

    // 2. BM25 ordering: Doc 1 (title match) harus rank #1 dengan score lebih tinggi dari Doc 2 (content only)
    let first = &body.items[0];
    let second = &body.items[1];
    assert_eq!(
        first.title, "Rust Ownership Guide",
        "Title boost harus membuat dokumen dengan match title teratas"
    );
    assert!(
        first.score > second.score,
        "Skor dokumen #1 ({}) harus lebih tinggi dari #2 ({})",
        first.score,
        second.score
    );

    // 3. Highlight dan line number verification
    let engine_hit = body
        .items
        .iter()
        .find(|item| item.relative_path == "src/engine.rs")
        .expect("src/engine.rs harus ditemukan di hasil pencarian");

    assert!(
        !engine_hit.highlights.is_empty(),
        "Highlights harus dikembalikan"
    );
    for hl in &engine_hit.highlights {
        assert!(hl.snippet.contains("<em>"));
        assert!(hl.snippet.contains("</em>"));
        assert!(hl.line_number.is_some(), "Line number harus terhitung");
    }

    let line_numbers: Vec<usize> = engine_hit
        .highlights
        .iter()
        .filter_map(|hl| hl.line_number)
        .collect();
    assert!(
        line_numbers.contains(&2),
        "Harus memuat baris 2: {line_numbers:?}"
    );
    assert!(
        line_numbers.contains(&4),
        "Harus memuat baris 4: {line_numbers:?}"
    );

    // 4. Pagination verification
    let req_p1 = Request::builder()
        .uri("/api/search?q=ownership&page=1&size=2")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_p1 = app.clone().oneshot(req_p1).await.unwrap();
    assert_eq!(res_p1.status(), StatusCode::OK);
    let p1_bytes = axum::body::to_bytes(res_p1.into_body(), usize::MAX)
        .await
        .unwrap();
    let p1_body: SearchResponseDto = serde_json::from_slice(&p1_bytes).unwrap();
    assert_eq!(p1_body.page, 1);
    assert_eq!(p1_body.size, 2);
    assert_eq!(p1_body.items.len(), 2);

    let req_p2 = Request::builder()
        .uri("/api/search?q=ownership&page=2&size=2")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_p2 = app.oneshot(req_p2).await.unwrap();
    assert_eq!(res_p2.status(), StatusCode::OK);
    let p2_bytes = axum::body::to_bytes(res_p2.into_body(), usize::MAX)
        .await
        .unwrap();
    let p2_body: SearchResponseDto = serde_json::from_slice(&p2_bytes).unwrap();
    assert_eq!(p2_body.page, 2);
    assert_eq!(p2_body.size, 2);
    assert_eq!(p2_body.items.len(), 1);
    assert_ne!(p1_body.items[0].id, p2_body.items[0].id);
}

#[tokio::test]
async fn test_phase_5_gate_in_memory() {
    let (state, _rx) = AppState::test_state();
    let folder_id = FolderId::new();
    let (doc1, doc2, doc3) = create_test_documents(folder_id);

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

    let app = backend::create_router_with_state(state);
    verify_search_gate_assertions(app).await;
}

#[tokio::test]
async fn test_phase_5_gate_live_elasticsearch() {
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
    let test_alias = format!("lynx_phase5_alias_{test_suffix}");
    let test_index = format!("lynx_phase5_index_{test_suffix}");

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
    let (doc1, doc2, doc3) = create_test_documents(folder_id);

    es_repo.index_document(&doc1).await.unwrap();
    es_repo.index_document(&doc2).await.unwrap();
    es_repo.index_document(&doc3).await.unwrap();

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

    let app = backend::create_router_with_state(state);

    // 3. Verifikasi seluruh assertions Gate Phase 5 pada live Elasticsearch
    verify_search_gate_assertions(app).await;

    // 4. Bersihkan index uji
    let _ = client
        .indices()
        .delete(IndicesDeleteParts::Index(&[&test_index]))
        .send()
        .await;
}
