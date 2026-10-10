use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::dtos::SearchResponseDto;
use backend::config::DEFAULT_ELASTICSEARCH_URL;
use backend::domain::models::{DocumentId, DocumentType, FolderId, IndexedDocument, Language};
use backend::domain::ports::SearchRepository;
use backend::domain::services::{DocumentExtractor, ExtractOptions, ExtractionResult, SkipReason};
use backend::infrastructure::elasticsearch::{
    DEFAULT_PING_TIMEOUT, EsSearchRepository, create_es_client_with_timeout,
    document_index_schema_with_alias, ping_elasticsearch_with_timeout,
};
use backend::state::{AppState, Repositories};
use chrono::Utc;
use elasticsearch::indices::{IndicesCreateParts, IndicesDeleteParts, IndicesRefreshParts};
use tower::ServiceExt;
use uuid::Uuid;

/// 9.9 Checklist Verification: Language and Type Detection
#[test]
fn test_9_9_language_and_type_detection() {
    let extractor = DocumentExtractor::new();
    let opts = ExtractOptions::default();

    let cases = [
        ("src/main.rs", DocumentType::Code, Some(Language::Rust)),
        ("src/app.ts", DocumentType::Code, Some(Language::Typescript)),
        (
            "src/app.tsx",
            DocumentType::Code,
            Some(Language::Typescript),
        ),
        (
            "src/index.js",
            DocumentType::Code,
            Some(Language::Javascript),
        ),
        ("main.py", DocumentType::Code, Some(Language::Python)),
        ("main.go", DocumentType::Code, Some(Language::Go)),
        ("Main.java", DocumentType::Code, Some(Language::Java)),
        ("App.kt", DocumentType::Code, Some(Language::Kotlin)),
        ("main.c", DocumentType::Code, Some(Language::C)),
        ("main.cpp", DocumentType::Code, Some(Language::Cpp)),
        ("README.md", DocumentType::Doc, Some(Language::Markdown)),
        ("notes.txt", DocumentType::Doc, Some(Language::Text)),
    ];

    for (filepath, expected_type, expected_lang) in cases {
        let res = extractor.extract(Path::new(filepath), b"print(42)", 9, None, &opts);
        match res {
            ExtractionResult::Extracted(doc) => {
                assert_eq!(doc.doc_type, expected_type, "Failed type for {filepath}");
                assert_eq!(
                    doc.language, expected_lang,
                    "Failed language for {filepath}"
                );
            }
            other => panic!("Expected Extracted for {filepath}, got {other:?}"),
        }
    }
}

/// 9.9 Checklist Verification: Config File Indexing
#[test]
fn test_9_9_config_file_indexing() {
    let extractor = DocumentExtractor::new();
    let opts = ExtractOptions::default();

    let config_cases = [
        ("config.json", Language::Json),
        ("Cargo.toml", Language::Toml),
        ("docker-compose.yaml", Language::Yaml),
        ("app.yml", Language::Yaml),
        ("settings.ini", Language::Ini),
    ];

    for (filepath, expected_lang) in config_cases {
        let res = extractor.extract(Path::new(filepath), b"key: value", 10, None, &opts);
        match res {
            ExtractionResult::Extracted(doc) => {
                assert_eq!(
                    doc.doc_type,
                    DocumentType::Config,
                    "Failed doc_type config for {filepath}"
                );
                assert_eq!(
                    doc.language,
                    Some(expected_lang),
                    "Failed language for {filepath}"
                );
            }
            other => panic!("Expected Extracted config for {filepath}, got {other:?}"),
        }
    }
}

/// 9.9 Checklist Verification: Secret File Rejection
#[test]
fn test_9_9_secret_file_rejection() {
    let extractor = DocumentExtractor::new();
    let opts = ExtractOptions::default();

    let secret_files = [
        ".env",
        ".env.local",
        ".env.production",
        "id_rsa",
        "id_rsa.pub",
        "id_ed25519",
        "server.pem",
        "tls.key",
        "keystore.p12",
        "cert.pfx",
        "secret.json",
        "credentials.yaml",
        "access_token.txt",
    ];

    for secret in secret_files {
        let res = extractor.extract(Path::new(secret), b"secret_token_12345", 18, None, &opts);
        assert!(
            matches!(res, ExtractionResult::Skipped(SkipReason::SecretFile(_))),
            "File {secret} was not skipped as a secret file!"
        );
    }
}

/// 9.9 Checklist Verification: Unsupported Language Fallback
#[test]
fn test_9_9_unsupported_language_fallback() {
    let extractor = DocumentExtractor::new();
    let opts = ExtractOptions::default();

    // Plain unknown text extension
    let res = extractor.extract(
        Path::new("scripts/custom_tool.unknownext"),
        b"echo custom command",
        19,
        None,
        &opts,
    );

    match res {
        ExtractionResult::Extracted(doc) => {
            // Falls back to Doc with Language::Text fallback without failing
            assert_eq!(doc.doc_type, DocumentType::Doc);
            assert_eq!(doc.extension, Some("unknownext".to_string()));
            assert_eq!(doc.language, Some(Language::Text));
        }
        other => panic!("Expected Extracted fallback for unknown extension, got {other:?}"),
    }
}

/// 9.9 Checklist Verification: Live ES Code Search Suite
/// Tests:
/// - `authenticate user` -> `authenticateUser`
/// - `authenticate user` -> `authenticate_user`
/// - Exact identifier outranks fuzzy alternatives
/// - Code snippet line number and indentation preservation
#[tokio::test]
async fn test_9_9_code_search_mastery_and_ranking_live_es() {
    let client =
        match create_es_client_with_timeout(DEFAULT_ELASTICSEARCH_URL, Duration::from_secs(15)) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping live ES test: {e}");
                return;
            }
        };

    if ping_elasticsearch_with_timeout(&client, DEFAULT_PING_TIMEOUT)
        .await
        .is_err()
    {
        eprintln!("Elasticsearch not reachable at {DEFAULT_ELASTICSEARCH_URL}, skipping");
        return;
    }

    let test_suffix = Uuid::new_v4().simple().to_string();
    let test_alias = format!("lynx_phase9_9_alias_{test_suffix}");
    let test_index = format!("lynx_phase9_9_index_{test_suffix}");

    // Create index with official document schema
    let schema = document_index_schema_with_alias(&test_alias);
    let create_res = client
        .indices()
        .create(IndicesCreateParts::Index(&test_index))
        .body(schema)
        .send()
        .await
        .expect("Create test index failed");
    assert!(create_res.status_code().is_success());

    let es_repo = Arc::new(EsSearchRepository::new(client.clone(), test_alias.clone()));
    let folder_id = FolderId::new();
    let now = Utc::now();

    // 1. Doc Snake (Rust)
    let doc_snake = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/security.rs"),
        folder_id,
        relative_path: "src/security.rs".to_string(),
        absolute_path: "/repo/src/security.rs".to_string(),
        title: "Security Controller".to_string(),
        content: "pub struct Security;\nimpl Security {\n    pub fn authenticate_user(id: u64) -> bool {\n        id > 0\n    }\n}".to_string(),
        tags: vec!["auth".to_string()],
        extension: Some("rs".to_string()),
        language: Some(Language::Rust),
        doc_type: DocumentType::Code,
        project: Some("backend".to_string()),
        file_size_bytes: 140,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash_snake".to_string()),
    };

    // 2. Doc Camel (TS)
    let doc_camel = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/auth.ts"),
        folder_id,
        relative_path: "src/auth.ts".to_string(),
        absolute_path: "/repo/src/auth.ts".to_string(),
        title: "Auth Handler".to_string(),
        content: "class AuthHandler {\n  authenticateUser(token: string): boolean {\n    return token !== '';\n  }\n}".to_string(),
        tags: vec!["auth".to_string()],
        extension: Some("ts".to_string()),
        language: Some(Language::Typescript),
        doc_type: DocumentType::Code,
        project: Some("frontend".to_string()),
        file_size_bytes: 120,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash_camel".to_string()),
    };

    // 3. Doc Markdown (Free text)
    let doc_free_text = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/login.md"),
        folder_id,
        relative_path: "docs/login.md".to_string(),
        absolute_path: "/repo/docs/login.md".to_string(),
        title: "Login Flow Guide".to_string(),
        content: "# Login Flow\nIn this system, we authenticate user sessions with JWT."
            .to_string(),
        tags: vec!["guide".to_string()],
        extension: Some("md".to_string()),
        language: Some(Language::Markdown),
        doc_type: DocumentType::Doc,
        project: Some("docs".to_string()),
        file_size_bytes: 90,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash_md".to_string()),
    };

    for doc in [&doc_snake, &doc_camel, &doc_free_text] {
        es_repo.index_document(doc).await.unwrap();
    }

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

    // Test A: Free text `authenticate user` matches both authenticateUser and authenticate_user
    let req = Request::builder()
        .uri("/api/search?q=authenticate%20user")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.total, 3);

    // Test B: Exact `authenticate_user` outranks fuzzy/other matches
    let req = Request::builder()
        .uri("/api/search?q=authenticate_user")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.items[0].relative_path, "src/security.rs");

    // Test C: Exact `authenticateUser` outranks fuzzy/other matches
    let req = Request::builder()
        .uri("/api/search?q=authenticateUser")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.items[0].relative_path, "src/auth.ts");

    // Test D: Code snippet line number and indentation verification
    let snake_hit = body
        .items
        .iter()
        .find(|item| item.relative_path == "src/security.rs")
        .expect("security.rs hit found");
    let hl = snake_hit
        .highlights
        .iter()
        .find(|h| h.snippet.contains("authenticate_user"))
        .expect("Highlight snippet found");

    // Line number 3: "    pub fn authenticate_user(id: u64) -> bool {"
    assert_eq!(hl.line_number, Some(3));
    assert!(hl.snippet.starts_with("    "));

    // Cleanup
    let _ = client
        .indices()
        .delete(IndicesDeleteParts::Index(&[&test_index]))
        .send()
        .await;
}
