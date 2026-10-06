use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::dtos::SearchResponseDto;
use backend::domain::models::{DocumentId, DocumentType, FolderId, IndexedDocument, Language};
use backend::state::AppState;
use chrono::Utc;
use tower::ServiceExt;

/// Helper untuk menginisialisasi router dengan dokumen uji in-memory.
async fn setup_test_search_app() -> (axum::Router, AppState, FolderId) {
    let (state, _rx) = AppState::test_state();
    let folder_id = FolderId::new();
    let now = Utc::now();

    // Doc 1: "Rust Ownership" di Title dan Content
    let doc1 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/ownership.md"),
        folder_id,
        relative_path: "docs/ownership.md".to_string(),
        absolute_path: "/workspace/docs/ownership.md".to_string(),
        title: "Rust Ownership and Borrowing Guide".to_string(),
        content: "Memory management in Rust without garbage collector.\nOwnership rules are strictly enforced by compiler.\nEvery value in Rust has an owner."
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

    // Doc 2: "Rust ownership" hanya di Content (Title tidak memiliki keyword)
    let doc2 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/memory.md"),
        folder_id,
        relative_path: "docs/memory.md".to_string(),
        absolute_path: "/workspace/docs/memory.md".to_string(),
        title: "Memory Allocation Internals".to_string(),
        content: "The Rust ownership model governs heap and stack memory allocation.\nSafe concurrency without data races."
            .to_string(),
        tags: vec!["memory".to_string(), "concurrency".to_string()],
        extension: Some("md".to_string()),
        language: None,
        doc_type: DocumentType::Doc,
        project: Some("core".to_string()),
        file_size_bytes: 2048,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash2".to_string()),
    };

    // Doc 3: Kode sumber dengan multiple fragments dan line number spesifik
    let doc3 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/fragments.rs"),
        folder_id,
        relative_path: "src/fragments.rs".to_string(),
        absolute_path: "/workspace/src/fragments.rs".to_string(),
        title: "Multi Fragment Demo".to_string(),
        content: "line 1: preamble\nline 2: first ownership mention here\nline 3: intermediate statement\nline 4: second ownership mention here\nline 5: end of file"
            .to_string(),
        tags: vec!["demo".to_string()],
        extension: Some("rs".to_string()),
        language: Some(Language::Rust),
        doc_type: DocumentType::Code,
        project: Some("examples".to_string()),
        file_size_bytes: 512,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash3".to_string()),
    };

    // Doc 4: Berkas lain (Python) tanpa keyword rust/ownership
    let doc4 = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "scripts/intro.py"),
        folder_id,
        relative_path: "scripts/intro.py".to_string(),
        absolute_path: "/workspace/scripts/intro.py".to_string(),
        title: "Python Script Intro".to_string(),
        content: "print('Python uses reference counting and garbage collector')".to_string(),
        tags: vec!["python".to_string()],
        extension: Some("py".to_string()),
        language: Some(Language::Python),
        doc_type: DocumentType::Code,
        project: Some("scripts".to_string()),
        file_size_bytes: 4096,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash4".to_string()),
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
    state
        .repositories
        .search
        .index_document(&doc4)
        .await
        .unwrap();

    let app = backend::create_router_with_state(state.clone());
    (app, state, folder_id)
}

#[tokio::test]
async fn test_search_rust_ownership_matches_title_and_content() {
    let (app, _, _) = setup_test_search_app().await;

    let req = Request::builder()
        .uri("/api/search?q=rust%20ownership")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body.query, "rust ownership");
    assert_eq!(body.total, 3);
    assert_eq!(body.items.len(), 3);
    assert_eq!(body.results.len(), 3);
    let _ = body.took_ms;

    for item in &body.items {
        assert!(item.score > 0.0, "Score harus dikembalikan dan > 0");
    }
}

#[tokio::test]
async fn test_title_boost_affects_rank() {
    let (app, _, _) = setup_test_search_app().await;

    // Doc 1 memiliki "Rust" dan "Ownership" di Title, sedangkan Doc 2 hanya di Content
    let req = Request::builder()
        .uri("/api/search?q=rust%20ownership")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body.items.len(), 3);
    let first = &body.items[0];
    let second = &body.items[1];

    assert_eq!(
        first.title, "Rust Ownership and Borrowing Guide",
        "Dokumen dengan match pada Title harus teranking lebih tinggi karena title boost"
    );
    assert!(
        first.score > second.score,
        "Skor dokumen dengan title match ({}) harus lebih tinggi dari content-only ({})",
        first.score,
        second.score
    );
}

#[tokio::test]
async fn test_search_pagination_works() {
    let (app, _, _) = setup_test_search_app().await;

    // Halaman 1 (size = 2)
    let req1 = Request::builder()
        .uri("/api/search?q=ownership&page=1&size=2")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);

    let body_bytes1 = axum::body::to_bytes(res1.into_body(), usize::MAX)
        .await
        .unwrap();
    let body1: SearchResponseDto = serde_json::from_slice(&body_bytes1).unwrap();

    assert_eq!(body1.page, 1);
    assert_eq!(body1.size, 2);
    assert_eq!(body1.total, 3);
    assert_eq!(body1.items.len(), 2);

    // Halaman 2 (size = 2)
    let req2 = Request::builder()
        .uri("/api/search?q=ownership&page=2&size=2")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);

    let body_bytes2 = axum::body::to_bytes(res2.into_body(), usize::MAX)
        .await
        .unwrap();
    let body2: SearchResponseDto = serde_json::from_slice(&body_bytes2).unwrap();

    assert_eq!(body2.page, 2);
    assert_eq!(body2.size, 2);
    assert_eq!(body2.total, 3);
    assert_eq!(body2.items.len(), 1);

    // Pastikan item halaman 2 berbeda dari halaman 1
    assert_ne!(body1.items[0].id, body2.items[0].id);
    assert_ne!(body1.items[1].id, body2.items[0].id);
}

#[tokio::test]
async fn test_empty_query_handled_without_error() {
    let (app, _, _) = setup_test_search_app().await;

    // 1. Tanpa parameter q
    let req = Request::builder()
        .uri("/api/search")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body.query, "");
    assert_eq!(body.total, 4, "Empty query harus match_all seluruh dokumen");
    assert_eq!(body.items.len(), 4);

    // 2. Query hanya whitespace
    let req_whitespace = Request::builder()
        .uri("/api/search?q=%20%20%20")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res_ws = app.oneshot(req_whitespace).await.unwrap();
    assert_eq!(res_ws.status(), StatusCode::OK);

    let body_bytes_ws = axum::body::to_bytes(res_ws.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_ws: SearchResponseDto = serde_json::from_slice(&body_bytes_ws).unwrap();

    assert_eq!(body_ws.query, "");
    assert_eq!(body_ws.total, 4);
}

#[tokio::test]
async fn test_case_insensitive_search() {
    let (app, _, _) = setup_test_search_app().await;

    let req_upper = Request::builder()
        .uri("/api/search?q=RUST%20OWNERSHIP")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res_upper = app.oneshot(req_upper).await.unwrap();
    assert_eq!(res_upper.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res_upper.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body.total, 3);
    assert_eq!(body.items.len(), 3);
}

#[tokio::test]
async fn test_special_characters_handled_safely() {
    let (app, _, _) = setup_test_search_app().await;

    // Karakter spesial regex/DSL Elasticsearch tidak boleh membuat server 500 / panic
    let req = Request::builder()
        .uri("/api/search?q=rust%20%26%20%26%20%2F%20*%20(%20)%20%3A%20%3B%20~%20%5E")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.total >= 1);
    let _ = body.took_ms;
}

#[tokio::test]
async fn test_highlights_multiple_fragments_and_line_numbers() {
    let (app, _, _) = setup_test_search_app().await;

    let req = Request::builder()
        .uri("/api/search?q=ownership")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    // Cari item Doc 3 (src/fragments.rs)
    let doc3_hit = body
        .items
        .iter()
        .find(|item| item.relative_path == "src/fragments.rs")
        .expect("src/fragments.rs harus ditemukan");

    // Verifikasi highlight muncul dan memiliki tag <em>
    assert!(!doc3_hit.highlights.is_empty(), "Highlight harus ada");
    for hl in &doc3_hit.highlights {
        assert!(hl.snippet.contains("<em>"));
        assert!(hl.snippet.contains("</em>"));
        assert!(hl.line_number.is_some(), "Line number harus terhitung");
    }

    // Verifikasi multiple fragments muncul (baris 2 dan baris 4)
    let line_numbers: Vec<usize> = doc3_hit
        .highlights
        .iter()
        .filter_map(|hl| hl.line_number)
        .collect();

    assert!(
        line_numbers.contains(&2),
        "Harus terdapat highlight baris 2: {line_numbers:?}"
    );
    assert!(
        line_numbers.contains(&4),
        "Harus terdapat highlight baris 4: {line_numbers:?}"
    );
}

#[tokio::test]
async fn test_result_dto_contains_display_metadata_and_dual_fields() {
    let (app, _, _) = setup_test_search_app().await;

    let req = Request::builder()
        .uri("/api/search?q=ownership")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&body_bytes).unwrap();

    // Verifikasi dual compatibility field items dan results
    assert_eq!(body.items, body.results);
    assert!(body.warnings.is_empty());

    let item = &body.items[0];
    assert!(!item.id.to_string().is_empty());
    assert!(!item.title.is_empty());
    assert!(!item.relative_path.is_empty());
    assert!(item.file_size > 0);
    assert!(item.updated_at.is_some());
}

#[tokio::test]
async fn test_page_and_size_validation_errors() {
    let (app, _, _) = setup_test_search_app().await;

    // 1. page = 0 -> 422 Unprocessable Entity
    let req_bad_page = Request::builder()
        .uri("/api/search?page=0")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_page = app.clone().oneshot(req_bad_page).await.unwrap();
    assert_eq!(res_page.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 2. size = 0 -> 422 Unprocessable Entity
    let req_bad_size_min = Request::builder()
        .uri("/api/search?size=0")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_size_min = app.clone().oneshot(req_bad_size_min).await.unwrap();
    assert_eq!(res_size_min.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 3. size = 101 -> 422 Unprocessable Entity
    let req_bad_size_max = Request::builder()
        .uri("/api/search?size=101")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_size_max = app.oneshot(req_bad_size_max).await.unwrap();
    assert_eq!(res_size_max.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
