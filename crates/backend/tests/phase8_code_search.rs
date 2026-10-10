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
use elasticsearch::indices::{
    IndicesAnalyzeParts, IndicesCreateParts, IndicesDeleteParts, IndicesRefreshParts,
};
use serde_json::json;
use std::path::Path;
use tower::ServiceExt;
use uuid::Uuid;

#[test]
fn test_9_1_canonical_code_mapping_and_secrets() {
    let extractor = DocumentExtractor::new();
    let opts = ExtractOptions::default();

    // 1. All code/config mappings from Phase 0
    let test_cases = [
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
        (
            "src/index.jsx",
            DocumentType::Code,
            Some(Language::Javascript),
        ),
        ("scripts/run.py", DocumentType::Code, Some(Language::Python)),
        ("cmd/main.go", DocumentType::Code, Some(Language::Go)),
        ("src/Main.java", DocumentType::Code, Some(Language::Java)),
        ("src/App.kt", DocumentType::Code, Some(Language::Kotlin)),
        ("src/main.c", DocumentType::Code, Some(Language::C)),
        ("src/main.cpp", DocumentType::Code, Some(Language::Cpp)),
        ("include/header.h", DocumentType::Code, Some(Language::C)),
        ("src/Program.cs", DocumentType::Code, Some(Language::Csharp)),
        ("lib/app.rb", DocumentType::Code, Some(Language::Ruby)),
        ("index.php", DocumentType::Code, Some(Language::Php)),
        (
            "Sources/App.swift",
            DocumentType::Code,
            Some(Language::Swift),
        ),
        ("deploy.sh", DocumentType::Code, Some(Language::Shell)),
        (
            "migrations/init.sql",
            DocumentType::Code,
            Some(Language::Sql),
        ),
        ("index.html", DocumentType::Code, Some(Language::Html)),
        ("styles.css", DocumentType::Code, Some(Language::Css)),
        ("theme.scss", DocumentType::Code, Some(Language::Scss)),
        ("App.vue", DocumentType::Code, Some(Language::Vue)),
        ("App.svelte", DocumentType::Code, Some(Language::Svelte)),
        ("init.lua", DocumentType::Code, Some(Language::Lua)),
        ("config.json", DocumentType::Config, Some(Language::Json)),
        ("Cargo.toml", DocumentType::Config, Some(Language::Toml)),
        (
            "docker-compose.yaml",
            DocumentType::Config,
            Some(Language::Yaml),
        ),
        ("service.yml", DocumentType::Config, Some(Language::Yaml)),
        ("setup.ini", DocumentType::Config, Some(Language::Ini)),
        ("README.md", DocumentType::Doc, Some(Language::Markdown)),
        ("notes.txt", DocumentType::Doc, Some(Language::Text)),
    ];

    for (filepath, expected_type, expected_lang) in test_cases {
        let res = extractor.extract(
            Path::new(filepath),
            b"code or config content",
            22,
            None,
            &opts,
        );
        match res {
            ExtractionResult::Extracted(doc) => {
                assert_eq!(
                    doc.doc_type, expected_type,
                    "Expected {expected_type:?} for {filepath}"
                );
                assert_eq!(
                    doc.language, expected_lang,
                    "Expected {expected_lang:?} for {filepath}"
                );
            }
            other => panic!("Expected extracted doc for {filepath}, got {other:?}"),
        }
    }

    // 2. Secret patterns are never indexed
    let secret_patterns = [
        ".env",
        ".env.local",
        ".env.production",
        "id_rsa",
        "id_rsa.pub",
        "id_dsa",
        "id_ed25519",
        "server.pem",
        "tls.key",
        "keystore.p12",
        "cert.pfx",
        "secret.json",
        "credentials.yaml",
        "token.txt",
    ];

    for secret_path in secret_patterns {
        let res = extractor.extract(Path::new(secret_path), b"secret_value", 12, None, &opts);
        assert!(
            matches!(res, ExtractionResult::Skipped(SkipReason::SecretFile(_))),
            "Expected secret {secret_path} to be skipped"
        );
    }
}

#[tokio::test]
async fn test_phase8_code_search_mastery_live_elasticsearch() {
    let client =
        match create_es_client_with_timeout(DEFAULT_ELASTICSEARCH_URL, Duration::from_secs(15)) {
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
        eprintln!("Elasticsearch not reachable at {DEFAULT_ELASTICSEARCH_URL}, skipping");
        return;
    }

    let test_suffix = Uuid::new_v4().simple().to_string();
    let test_alias = format!("lynx_phase8_alias_{test_suffix}");
    let test_index = format!("lynx_phase8_index_{test_suffix}");

    // ------------------------------------------------------------------------
    // 9.2 Code analyzer live verification
    // ------------------------------------------------------------------------
    let schema = document_index_schema_with_alias(&test_alias);
    let create_res = client
        .indices()
        .create(IndicesCreateParts::Index(&test_index))
        .body(schema)
        .send()
        .await
        .expect("Create test index failed");
    assert!(create_res.status_code().is_success());

    // Verify code_subword_filter with camelCase
    let camel_res = client
        .indices()
        .analyze(IndicesAnalyzeParts::Index(&test_index))
        .body(json!({
            "analyzer": "code_analyzer",
            "text": "authenticateUser"
        }))
        .send()
        .await
        .expect("Analyze camelCase failed");
    assert!(camel_res.status_code().is_success());
    let camel_json: serde_json::Value = camel_res.json().await.unwrap();
    let camel_tokens: Vec<String> = camel_json["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["token"].as_str().map(String::from))
        .collect();

    assert!(
        camel_tokens.contains(&"authenticateuser".to_string()),
        "original token preservation: {camel_tokens:?}"
    );
    assert!(
        camel_tokens.contains(&"authenticate".to_string()),
        "camelCase split part 1: {camel_tokens:?}"
    );
    assert!(
        camel_tokens.contains(&"user".to_string()),
        "camelCase split part 2: {camel_tokens:?}"
    );

    // Verify code_subword_filter with snake_case
    let snake_res = client
        .indices()
        .analyze(IndicesAnalyzeParts::Index(&test_index))
        .body(json!({
            "analyzer": "code_analyzer",
            "text": "authenticate_user"
        }))
        .send()
        .await
        .expect("Analyze snake_case failed");
    assert!(snake_res.status_code().is_success());
    let snake_json: serde_json::Value = snake_res.json().await.unwrap();
    let snake_tokens: Vec<String> = snake_json["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["token"].as_str().map(String::from))
        .collect();

    assert!(
        snake_tokens.contains(&"authenticate_user".to_string()),
        "original token preservation: {snake_tokens:?}"
    );
    assert!(
        snake_tokens.contains(&"authenticate".to_string()),
        "snake_case split part 1: {snake_tokens:?}"
    );
    assert!(
        snake_tokens.contains(&"user".to_string()),
        "snake_case split part 2: {snake_tokens:?}"
    );

    // Verify numeric splitting behavior
    let num_res = client
        .indices()
        .analyze(IndicesAnalyzeParts::Index(&test_index))
        .body(json!({
            "analyzer": "code_analyzer",
            "text": "token123"
        }))
        .send()
        .await
        .expect("Analyze numeric failed");
    assert!(num_res.status_code().is_success());
    let num_json: serde_json::Value = num_res.json().await.unwrap();
    let num_tokens: Vec<String> = num_json["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["token"].as_str().map(String::from))
        .collect();

    assert!(num_tokens.contains(&"token123".to_string()));
    assert!(num_tokens.contains(&"token".to_string()));
    assert!(num_tokens.contains(&"123".to_string()));

    // ------------------------------------------------------------------------
    // Index test documents for ranking, code search query logic, and highlight
    // ------------------------------------------------------------------------
    let es_repo = Arc::new(EsSearchRepository::new(client.clone(), test_alias.clone()));
    let folder_id = FolderId::new();
    let now = Utc::now();

    // Doc A (Rust): exact snake_case identifier with 4-space indentation
    let doc_a = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/auth.rs"),
        folder_id,
        relative_path: "src/auth.rs".to_string(),
        absolute_path: "/workspace/src/auth.rs".to_string(),
        title: "Rust Auth Module".to_string(),
        content: "pub mod auth {\n    pub fn authenticate_user(token: &str) -> bool {\n        !token.is_empty()\n    }\n}".to_string(),
        tags: vec!["auth".to_string()],
        extension: Some("rs".to_string()),
        language: Some(Language::Rust),
        doc_type: DocumentType::Code,
        project: Some("backend".to_string()),
        file_size_bytes: 120,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash_a".to_string()),
    };

    // Doc B (TS): exact camelCase identifier with 2-space indentation
    let doc_b = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "src/auth.ts"),
        folder_id,
        relative_path: "src/auth.ts".to_string(),
        absolute_path: "/workspace/src/auth.ts".to_string(),
        title: "TypeScript Auth Module".to_string(),
        content: "export class AuthService {\n  authenticateUser(token: string): boolean {\n    return token.length > 0;\n  }\n}".to_string(),
        tags: vec!["auth".to_string()],
        extension: Some("ts".to_string()),
        language: Some(Language::Typescript),
        doc_type: DocumentType::Code,
        project: Some("frontend".to_string()),
        file_size_bytes: 110,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash_b".to_string()),
    };

    // Doc C (Doc): free text "authenticate user" in documentation
    let doc_c = IndexedDocument {
        id: DocumentId::from_relative_path(folder_id, "docs/auth.md"),
        folder_id,
        relative_path: "docs/auth.md".to_string(),
        absolute_path: "/workspace/docs/auth.md".to_string(),
        title: "User Authentication Guide".to_string(),
        content: "# Guide\nThis guide explains how to authenticate user credentials across microservices.".to_string(),
        tags: vec!["docs".to_string()],
        extension: Some("md".to_string()),
        language: Some(Language::Markdown),
        doc_type: DocumentType::Doc,
        project: Some("docs".to_string()),
        file_size_bytes: 100,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("hash_c".to_string()),
    };

    for doc in [&doc_a, &doc_b, &doc_c] {
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

    // ------------------------------------------------------------------------
    // 9.2 & 9.3 Relevance & Query Logic
    // ------------------------------------------------------------------------
    // Test: Query `authenticate_user`
    let req = Request::builder()
        .uri("/api/search?q=authenticate_user")
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
        body.total >= 2,
        "Both doc_a and doc_b should match subwords"
    );
    // Ensure exact snake_case match (doc_a) has the strongest relevance
    assert_eq!(
        body.items[0].relative_path, "src/auth.rs",
        "Exact snake_case identifier match must be top hit"
    );

    // Test: Query `authenticateUser`
    let req = Request::builder()
        .uri("/api/search?q=authenticateUser")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: SearchResponseDto = serde_json::from_slice(&bytes).unwrap();
    assert!(body.total >= 2);
    // Ensure exact camelCase match (doc_b) has the strongest relevance
    assert_eq!(
        body.items[0].relative_path, "src/auth.ts",
        "Exact camelCase identifier match must be top hit"
    );

    // Test: Query free text `authenticate user`
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
    assert_eq!(
        body.total, 3,
        "All 3 documents must match 'authenticate user'"
    );

    // Test: Filters `language:rust` and `type:code` continue working with code terms
    let req = Request::builder()
        .uri("/api/search?q=authenticate_user%20language:rust%20type:code")
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
    assert_eq!(body.items[0].relative_path, "src/auth.rs");
    assert_eq!(body.items[0].language, Some(Language::Rust));
    assert_eq!(body.items[0].doc_type, DocumentType::Code);

    // ------------------------------------------------------------------------
    // 9.4 Code highlight verification
    // ------------------------------------------------------------------------
    let rust_hit = &body.items[0];
    let content_hl = rust_hit
        .highlights
        .iter()
        .find(|h| h.snippet.contains("authenticate_user"))
        .expect("Highlight for authenticate_user must be present");

    // 1. Matched identifier/term highlighted
    assert!(
        content_hl.snippet.contains("<em>"),
        "Identifier must be highlighted with <em>: {}",
        content_hl.snippet
    );

    // 2. Preserve indentation (4 spaces from raw content line 2)
    assert!(
        content_hl.snippet.starts_with("    "),
        "Indentation must be preserved: {:?}",
        content_hl.snippet
    );

    // 3. Return accurate line number pointing into raw source content
    assert_eq!(
        content_hl.line_number,
        Some(2),
        "Line number must point to line 2 in src/auth.rs"
    );

    // 4. Verify line number still points into raw source content
    let raw_line = doc_a
        .content
        .lines()
        .nth(content_hl.line_number.unwrap() - 1)
        .unwrap();
    assert!(
        raw_line.contains("pub fn authenticate_user"),
        "Line number points to raw source line: {raw_line}"
    );
    assert!(
        raw_line.starts_with("    "),
        "Raw line starts with 4 spaces"
    );

    // Clean up test index
    let _ = client
        .indices()
        .delete(IndicesDeleteParts::Index(&[&test_index]))
        .send()
        .await;
}
