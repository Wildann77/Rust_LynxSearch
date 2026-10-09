use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::dtos::SearchResponseDto;
use backend::config::DEFAULT_ELASTICSEARCH_URL;
use backend::domain::models::{DocumentId, DocumentType, FolderId, IndexedDocument};
use backend::domain::ports::SearchRepository;
use backend::infrastructure::elasticsearch::{
    DEFAULT_PING_TIMEOUT, EsSearchRepository, create_es_client_with_timeout,
    document_index_schema_with_alias, ping_elasticsearch_with_timeout,
};
use backend::state::AppState;
use chrono::Utc;
use elasticsearch::indices::{IndicesCreateParts, IndicesDeleteParts, IndicesRefreshParts};
use tower::ServiceExt;

/// Helper to set up the in-memory test router populated with documents
/// designed to test exact, fuzzy, and prefix scenarios.
async fn setup_fuzzy_prefix_test_app() -> (axum::Router, AppState, FolderId) {
    let (state, _rx) = AppState::test_state();
    let folder_id = FolderId::new();
    let now = Utc::now();

    // Doc 1: Exact "Rust Ownership"
    let doc1 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/ownership.md"),
        folder_id,
        relative_path: "docs/ownership.md".to_string(),
        absolute_path: "/workspace/docs/ownership.md".to_string(),
        title: "Rust Ownership and Borrowing Guide".to_string(),
        content: "Memory management in Rust without garbage collection.\nOwnership rules are strictly enforced by the compiler.\nEvery value in Rust has an owner."
            .to_string(),
        tags: vec!["rust".to_string(), "ownership".to_string()],
        extension: Some("md".to_string()),
        language: None,
        doc_type: DocumentType::Doc,
        project: Some("guide".to_string()),
        file_size_bytes: 1024,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash1".to_string()),
    };

    // Doc 2: Contains intentional typo "ownrship" in title & content
    let doc2 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/typo_note.md"),
        folder_id,
        relative_path: "docs/typo_note.md".to_string(),
        absolute_path: "/workspace/docs/typo_note.md".to_string(),
        title: "Common Typo in Rust Ownrship".to_string(),
        content: "Developers frequently misspell ownership as ownrship in code comments."
            .to_string(),
        tags: vec!["rust".to_string(), "ownrship".to_string()],
        extension: Some("md".to_string()),
        language: None,
        doc_type: DocumentType::Doc,
        project: Some("notes".to_string()),
        file_size_bytes: 512,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash2".to_string()),
    };

    // Doc 3: Unrelated topic
    let doc3 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/networking.md"),
        folder_id,
        relative_path: "docs/networking.md".to_string(),
        absolute_path: "/workspace/docs/networking.md".to_string(),
        title: "Asynchronous Networking with Tokio".to_string(),
        content: "High performance non-blocking sockets and timers in Tokio runtime.".to_string(),
        tags: vec!["tokio".to_string(), "async".to_string()],
        extension: Some("md".to_string()),
        language: None,
        doc_type: DocumentType::Doc,
        project: Some("net".to_string()),
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

    let router = backend::create_router_with_state(state.clone());
    (router, state, folder_id)
}

// ============================================================================
// 1. Task 8.8: Fuzzy Search Tests
// ============================================================================

#[tokio::test]
async fn test_fuzzy_search_matches_typo_rust_ownrship() {
    let (app, _, _) = setup_fuzzy_prefix_test_app().await;

    // Search with typo query: "rust ownrship"
    let req = Request::builder()
        .uri("/api/search?q=rust+ownrship")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    // Verify Doc 1 ("Rust Ownership and Borrowing Guide") is found despite query typo
    let matched_doc1 = body
        .items
        .iter()
        .find(|item| item.relative_path == "docs/ownership.md");
    assert!(
        matched_doc1.is_some(),
        "Query 'rust ownrship' must match 'Rust Ownership and Borrowing Guide'"
    );

    // Verify highlights are produced
    let doc1_item = matched_doc1.unwrap();
    assert!(
        !doc1_item.highlights.is_empty(),
        "Must produce highlight snippets for fuzzy matches"
    );
}

#[tokio::test]
async fn test_fuzzy_rank_ordering_exact_result_gets_stronger_score() {
    let (app, _, _) = setup_fuzzy_prefix_test_app().await;

    // Search with exact query: "rust ownership"
    let req = Request::builder()
        .uri("/api/search?q=rust+ownership")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.items.len() >= 2);

    let doc1_exact = body
        .items
        .iter()
        .find(|item| item.relative_path == "docs/ownership.md")
        .expect("docs/ownership.md must be returned");
    let doc2_typo = body
        .items
        .iter()
        .find(|item| item.relative_path == "docs/typo_note.md")
        .expect("docs/typo_note.md must be returned");

    // Exact result must score higher than the typo document
    assert!(
        doc1_exact.score > doc2_typo.score,
        "Exact match score ({}) must be strictly greater than fuzzy match score ({})",
        doc1_exact.score,
        doc2_typo.score
    );

    // Exact result must be the top rank
    assert_eq!(
        body.items[0].relative_path, "docs/ownership.md",
        "Exact result must be ranked at position 0"
    );
}

#[tokio::test]
async fn test_fuzzy_does_not_activate_on_facet_filter_values() {
    let (app, _, _) = setup_fuzzy_prefix_test_app().await;

    // Search with query "rust" but facet filter tag="ownrship" (typo)
    // Only Doc 2 actually has tag "ownrship". Doc 1 has tag "ownership".
    // Facet filtering MUST be exact and not fuzzified!
    let req = Request::builder()
        .uri("/api/search?q=rust&tag=ownrship")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    // Verify Doc 1 is NOT returned because its tag is "ownership", not "ownrship"
    let doc1_present = body
        .items
        .iter()
        .any(|item| item.relative_path == "docs/ownership.md");
    assert!(
        !doc1_present,
        "Fuzzy search must NOT match facet filter tag=ownrship against tag=ownership"
    );

    // Doc 2 must be returned because its tag exactly matches "ownrship"
    let doc2_present = body
        .items
        .iter()
        .any(|item| item.relative_path == "docs/typo_note.md");
    assert!(
        doc2_present,
        "Document with exact tag ownrship must be returned"
    );
}

// ============================================================================
// 2. Task 8.9: Prefix Search Tests
// ============================================================================

#[tokio::test]
async fn test_prefix_search_owner_finds_ownership() {
    let (app, _, _) = setup_fuzzy_prefix_test_app().await;

    // Search with prefix: "owner"
    let req = Request::builder()
        .uri("/api/search?q=owner")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    let doc1_present = body
        .items
        .iter()
        .any(|item| item.relative_path == "docs/ownership.md");
    assert!(
        doc1_present,
        "Prefix search 'owner' must match 'Rust Ownership and Borrowing Guide'"
    );
}

#[tokio::test]
async fn test_prefix_search_partial_term_borrow() {
    let (app, _, _) = setup_fuzzy_prefix_test_app().await;

    // Partial term: "borrow"
    let req = Request::builder()
        .uri("/api/search?q=borrow")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    let doc1_present = body
        .items
        .iter()
        .any(|item| item.relative_path == "docs/ownership.md");
    assert!(
        doc1_present,
        "Partial term 'borrow' must match 'Borrowing' in docs/ownership.md"
    );
}

#[tokio::test]
async fn test_prefix_search_no_match() {
    let (app, _, _) = setup_fuzzy_prefix_test_app().await;

    // Search with term that matches nothing
    let req = Request::builder()
        .uri("/api/search?q=nonexistentgibberishxyz")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body.items.len(), 0, "No results for nonexistent term");
    assert_eq!(body.total, 0);
}

// ============================================================================
// 3. Live Elasticsearch Integration Tests (TASK.md 8.8 & 8.9)
// ============================================================================

#[tokio::test]
async fn test_live_elasticsearch_fuzzy_and_prefix_rank_ordering() {
    let _ = dotenvy::dotenv();
    let es_url = std::env::var("ELASTICSEARCH_URL")
        .unwrap_or_else(|_| DEFAULT_ELASTICSEARCH_URL.to_string());

    let client = match create_es_client_with_timeout(&es_url, Duration::from_secs(10)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping live ES test: cannot initialize client: {e}");
            return;
        }
    };

    if ping_elasticsearch_with_timeout(&client, DEFAULT_PING_TIMEOUT)
        .await
        .is_err()
    {
        eprintln!("Elasticsearch not reachable at {es_url}, skipping live integration test");
        return;
    }

    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let test_alias = format!("lynx_test_alias_{test_suffix}");
    let test_index = format!("lynx_test_index_{test_suffix}");

    // 1. Create index with mapping schema
    let schema_payload = document_index_schema_with_alias(&test_alias);
    let create_res = client
        .indices()
        .create(IndicesCreateParts::Index(&test_index))
        .body(schema_payload)
        .send()
        .await
        .expect("Create index must succeed");
    assert!(create_res.status_code().is_success());

    let repo = EsSearchRepository::new(client.clone(), test_alias.clone());
    let folder_id = FolderId::new();
    let now = Utc::now();

    // Doc A: Exact "Rust Ownership"
    let doc_exact = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/ownership.md"),
        folder_id,
        relative_path: "docs/ownership.md".to_string(),
        absolute_path: "/workspace/docs/ownership.md".to_string(),
        title: "Rust Ownership and Borrowing Guide".to_string(),
        content: "Memory management in Rust without garbage collector. Ownership rules are strictly enforced."
            .to_string(),
        tags: vec!["rust".to_string(), "ownership".to_string()],
        extension: Some("md".to_string()),
        language: None,
        doc_type: DocumentType::Doc,
        project: Some("core".to_string()),
        file_size_bytes: 1024,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("h1".to_string()),
    };

    // Doc B: Typo "Rust Ownrship"
    let doc_typo = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/typo.md"),
        folder_id,
        relative_path: "docs/typo.md".to_string(),
        absolute_path: "/workspace/docs/typo.md".to_string(),
        title: "Common Typo in Rust Ownrship".to_string(),
        content: "Explaining why developers mistype ownrship in documentation.".to_string(),
        tags: vec!["rust".to_string(), "ownrship".to_string()],
        extension: Some("md".to_string()),
        language: None,
        doc_type: DocumentType::Doc,
        project: Some("core".to_string()),
        file_size_bytes: 512,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("h2".to_string()),
    };

    repo.index_document(&doc_exact).await.unwrap();
    repo.index_document(&doc_typo).await.unwrap();

    // Refresh index so documents are immediately searchable
    client
        .indices()
        .refresh(IndicesRefreshParts::Index(&[&test_index]))
        .send()
        .await
        .unwrap();

    // 2. Test 8.8: "rust ownrship" matches Doc A via Elasticsearch fuzzy query
    let query_typo = backend::domain::query_parser::QueryParser::parse("rust ownrship");
    let builder_typo =
        backend::domain::services::search_query_builder::SearchQueryBuilder::new(query_typo);
    let dsl_typo = builder_typo.build_raw().unwrap();

    let res_typo = repo.search(&dsl_typo).await.unwrap();
    let parsed_typo = res_typo.parse_execution_result().unwrap();

    let found_doc_exact = parsed_typo
        .hits
        .iter()
        .find(|h| h.relative_path == "docs/ownership.md");
    assert!(
        found_doc_exact.is_some(),
        "Live ES: 'rust ownrship' must match 'Rust Ownership and Borrowing Guide'"
    );

    // 3. Test 8.8: Exact result gets stronger score; fuzzy result never overrides exact match
    let query_exact = backend::domain::query_parser::QueryParser::parse("rust ownership");
    let builder_exact =
        backend::domain::services::search_query_builder::SearchQueryBuilder::new(query_exact);
    let dsl_exact = builder_exact.build_raw().unwrap();

    let res_exact = repo.search(&dsl_exact).await.unwrap();
    let parsed_exact = res_exact.parse_execution_result().unwrap();

    assert!(parsed_exact.hits.len() >= 2);
    let exact_hit = &parsed_exact.hits[0];
    let second_hit = &parsed_exact.hits[1];

    assert_eq!(
        exact_hit.relative_path, "docs/ownership.md",
        "Live ES: Exact match must be ranked #1"
    );
    assert!(
        exact_hit.score > second_hit.score,
        "Live ES: Exact match score ({}) must be strictly higher than typo score ({})",
        exact_hit.score,
        second_hit.score
    );

    // 4. Test 8.9: Prefix search 'owner' finds 'ownership'
    let query_prefix = backend::domain::query_parser::QueryParser::parse("owner");
    let builder_prefix =
        backend::domain::services::search_query_builder::SearchQueryBuilder::new(query_prefix);
    let dsl_prefix = builder_prefix.build_raw().unwrap();

    let res_prefix = repo.search(&dsl_prefix).await.unwrap();
    let parsed_prefix = res_prefix.parse_execution_result().unwrap();

    let found_prefix = parsed_prefix
        .hits
        .iter()
        .find(|h| h.relative_path == "docs/ownership.md");
    assert!(
        found_prefix.is_some(),
        "Live ES: Prefix 'owner' must match 'Rust Ownership and Borrowing Guide'"
    );

    // 5. Cleanup
    let _ = client
        .indices()
        .delete(IndicesDeleteParts::Index(&[&test_index]))
        .send()
        .await;
}
