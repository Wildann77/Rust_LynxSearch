use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::api::dtos::SearchResultItemDto;
use backend::api::routes::create_router_with_state;
use backend::config::{AppConfig, ConfigError};
use backend::domain::models::{DocumentId, DocumentStatus, DocumentType, Folder, RegistryEntry};
use backend::domain::ports::{FileWalker, WalkOptions};
use backend::domain::services::document_extractor::{
    DocumentExtractor, ExtractOptions, ExtractionResult, SkipReason,
};
use backend::infrastructure::fs::walker::{LocalFileWalker, is_secret_file};
use backend::state::AppState;
use backend::telemetry::{cleanup_old_logs, get_log_directory, sanitize_secret_url};
use chrono::Utc;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::tempdir;
use tower::ServiceExt;
use uuid::Uuid;

#[test]
fn test_section12_1_localhost_isolation_enforcement() {
    // 1. Forbid 0.0.0.0 in BACKEND_BIND_ADDR
    let err_bind = AppConfig::from_lookup(|k| match k {
        "DATABASE_URL" => Some("postgres://lynx:pass@127.0.0.1:5432/lynxsearch".to_string()),
        "BACKEND_BIND_ADDR" => Some("0.0.0.0:3001".to_string()),
        _ => None,
    })
    .unwrap_err();

    assert!(matches!(err_bind, ConfigError::SecurityViolation(_)));

    // 2. Forbid 0.0.0.0 in BACKEND_HOST
    let err_host = AppConfig::from_lookup(|k| match k {
        "DATABASE_URL" => Some("postgres://lynx:pass@127.0.0.1:5432/lynxsearch".to_string()),
        "BACKEND_HOST" => Some("0.0.0.0".to_string()),
        _ => None,
    })
    .unwrap_err();

    assert!(matches!(err_host, ConfigError::SecurityViolation(_)));

    // 3. Default binds strictly to 127.0.0.1:3001
    let cfg = AppConfig::from_lookup(|k| match k {
        "DATABASE_URL" => Some("postgres://lynx:pass@127.0.0.1:5432/lynxsearch".to_string()),
        _ => None,
    })
    .unwrap();

    assert_eq!(cfg.backend_host, "127.0.0.1");
    assert_eq!(cfg.backend_port, 3001);
    assert_eq!(cfg.backend_bind_addr, "127.0.0.1:3001");
}

#[tokio::test]
async fn test_section12_2_path_traversal_defense() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state.clone());

    // 1. Setup registered folder inside temp dir
    let inside_dir = tempdir().unwrap();
    let folder = Folder::new(inside_dir.path().to_path_buf());
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // Setup an external outside folder containing sensitive data
    let outside_dir = tempdir().unwrap();
    let outside_file = outside_dir.path().join("external_secret.txt");
    fs::write(&outside_file, "SENSITIVE_DATA_OUTSIDE").unwrap();

    // 2. Test Case A: Symlink escaping registered folder boundary
    let escaped_symlink = inside_dir.path().join("symlink_to_outside.txt");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside_file, &escaped_symlink).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&outside_file, &escaped_symlink).unwrap();

    let symlink_doc_id = DocumentId::from_uuid(Uuid::new_v4());
    let symlink_entry = RegistryEntry {
        id: symlink_doc_id,
        folder_id: folder.id,
        relative_path: "symlink_to_outside.txt".to_string(),
        content_hash: "dummyhash".to_string(),
        file_size: 20,
        status: DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state
        .repositories
        .registry
        .upsert_entry(&symlink_entry)
        .await
        .unwrap();

    let req = Request::builder()
        .uri(format!("/api/documents/{}", symlink_doc_id.as_uuid()))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let body = axum::body::to_bytes(res.into_body(), 1024 * 64)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "PATH_TRAVERSAL_DETECTED");

    // 3. Test Case B: Relative path containing `..`
    let dot_dot_doc_id = DocumentId::from_uuid(Uuid::new_v4());
    let dot_dot_entry = RegistryEntry {
        id: dot_dot_doc_id,
        folder_id: folder.id,
        relative_path: "../../../etc/passwd".to_string(),
        content_hash: "dummyhash2".to_string(),
        file_size: 20,
        status: DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state
        .repositories
        .registry
        .upsert_entry(&dot_dot_entry)
        .await
        .unwrap();

    let req = Request::builder()
        .uri(format!("/api/documents/{}", dot_dot_doc_id.as_uuid()))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let body = axum::body::to_bytes(res.into_body(), 1024 * 64)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "PATH_TRAVERSAL_DETECTED");

    // 4. Test Case C: Relative path containing null bytes `\0`
    let null_byte_doc_id = DocumentId::from_uuid(Uuid::new_v4());
    let null_byte_entry = RegistryEntry {
        id: null_byte_doc_id,
        folder_id: folder.id,
        relative_path: "document\0hidden.txt".to_string(),
        content_hash: "dummyhash3".to_string(),
        file_size: 20,
        status: DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state
        .repositories
        .registry
        .upsert_entry(&null_byte_entry)
        .await
        .unwrap();

    let req = Request::builder()
        .uri(format!("/api/documents/{}", null_byte_doc_id.as_uuid()))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let body = axum::body::to_bytes(res.into_body(), 1024 * 64)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "PATH_TRAVERSAL_DETECTED");

    // 5. Test Case D: Valid file inside boundary succeeds
    let valid_file = inside_dir.path().join("safe_notes.md");
    fs::write(&valid_file, "# Safe Documentation\nValid content.").unwrap();

    let valid_doc_id = DocumentId::from_uuid(Uuid::new_v4());
    let valid_entry = RegistryEntry {
        id: valid_doc_id,
        folder_id: folder.id,
        relative_path: "safe_notes.md".to_string(),
        content_hash: "validhash".to_string(),
        file_size: 40,
        status: DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state
        .repositories
        .registry
        .upsert_entry(&valid_entry)
        .await
        .unwrap();

    let req = Request::builder()
        .uri(format!("/api/documents/{}", valid_doc_id.as_uuid()))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 1024 * 64)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["title"], "Safe Documentation");
}

#[tokio::test]
async fn test_section12_3_secret_exclusion_verification() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    // Create secret files
    fs::write(root.join(".env"), "DATABASE_URL=postgres://...").unwrap();
    fs::write(root.join(".env.local"), "SECRET_KEY=12345").unwrap();
    fs::write(root.join("id_rsa"), "-----BEGIN RSA PRIVATE KEY-----").unwrap();
    fs::write(
        root.join("id_ed25519"),
        "-----BEGIN OPENSSH PRIVATE KEY-----",
    )
    .unwrap();
    fs::write(root.join("server.key"), "-----BEGIN PRIVATE KEY-----").unwrap();
    fs::write(root.join("cert.pem"), "-----BEGIN CERTIFICATE-----").unwrap();
    fs::write(root.join("cert.p12"), "p12 binary").unwrap();
    fs::write(root.join("secret_token.txt"), "secret token value").unwrap();

    // Create valid files (including a source file with token in its name)
    fs::write(root.join("token.rs"), "pub struct Token;").unwrap();
    fs::write(root.join("guide.md"), "# User Guide").unwrap();

    // 1. Verify walker exclusion
    let walker = LocalFileWalker::new();
    let options = WalkOptions::default();
    let discovered = walker
        .walk(root, &options)
        .unwrap()
        .map(|r| r.unwrap().relative_path.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    assert!(discovered.contains(&"token.rs".to_string()));
    assert!(discovered.contains(&"guide.md".to_string()));
    assert!(!discovered.contains(&".env".to_string()));
    assert!(!discovered.contains(&".env.local".to_string()));
    assert!(!discovered.contains(&"id_rsa".to_string()));
    assert!(!discovered.contains(&"id_ed25519".to_string()));
    assert!(!discovered.contains(&"server.key".to_string()));
    assert!(!discovered.contains(&"cert.pem".to_string()));
    assert!(!discovered.contains(&"cert.p12".to_string()));
    assert!(!discovered.contains(&"secret_token.txt".to_string()));

    // 2. Verify is_secret_file unit helper
    assert!(is_secret_file(Path::new(".env")));
    assert!(is_secret_file(Path::new("prod.key")));
    assert!(is_secret_file(Path::new("id_rsa")));
    assert!(!is_secret_file(Path::new("token.rs")));
    assert!(!is_secret_file(Path::new("tokenizer.py")));

    // 3. Verify DocumentExtractor skip reason
    let extractor = DocumentExtractor::new();
    let opts = ExtractOptions::default();
    let res = extractor.extract(
        Path::new(".env"),
        b"SECRET=xyz",
        10,
        Some(Utc::now()),
        &opts,
    );
    assert!(matches!(
        res,
        ExtractionResult::Skipped(SkipReason::SecretFile(_))
    ));
}

#[tokio::test]
async fn test_section12_4_api_exposure_and_cors_isolation() {
    let (state, _rx) = AppState::test_state();
    let app = create_router_with_state(state);

    // 1. Request from unauthorized third-party origin
    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .header("Origin", "https://malicious-website.com")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Unauthorized origin must NOT be reflected in Access-Control-Allow-Origin
    assert_ne!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://malicious-website.com")
    );

    // 2. Request from legitimate desktop origin
    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .header("Origin", "tauri://localhost")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("tauri://localhost")
    );

    // 3. Ensure SearchResultItemDto does not expose absolute disk paths
    let item = SearchResultItemDto {
        id: DocumentId::from_uuid(Uuid::new_v4()),
        title: "Test".to_string(),
        relative_path: "src/main.rs".to_string(),
        project: None,
        doc_type: DocumentType::Code,
        language: None,
        tags: vec![],
        highlights: vec![],
        score: 1.0,
        file_size: 100,
        updated_at: None,
    };
    let json_val = serde_json::to_value(&item).unwrap();
    assert!(json_val.get("absolute_path").is_none());
    assert_eq!(json_val["relative_path"], "src/main.rs");
}

#[test]
fn test_section12_5_and_6_log_safety_and_persistence() {
    // 1. Credential sanitization
    let db_url = "postgres://lynx_user:supersecret_pw@127.0.0.1:5432/lynxsearch";
    let sanitized = sanitize_secret_url(db_url);
    assert_eq!(
        sanitized,
        "postgres://lynx_user:***@127.0.0.1:5432/lynxsearch"
    );

    // 2. Log directory resolution conforms to OS config path
    let log_dir = get_log_directory();
    assert!(log_dir.ends_with(PathBuf::from("lynxsearch").join("logs")));

    // 3. Cleanup of log files older than 7 days
    let temp = tempdir().unwrap();
    let old_log = temp.path().join("lynxsearch.log.2024-01-01");
    let fresh_log = temp.path().join("lynxsearch.log.2026-10-09");
    fs::write(&old_log, "stale log entry").unwrap();
    fs::write(&fresh_log, "fresh log entry").unwrap();

    // Prune with 0 duration will prune files
    let pruned = cleanup_old_logs(temp.path(), Duration::from_secs(0)).unwrap();
    assert_eq!(pruned, 2);
    assert!(!old_log.exists());
    assert!(!fresh_log.exists());
}
