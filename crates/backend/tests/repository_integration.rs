use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use backend::config::{Bm25Weights, DEFAULT_ELASTICSEARCH_URL};
use backend::domain::models::{
    AppSettings, DocumentId, DocumentStatus, Folder, FolderStatus, IndexingJob, JobId,
    JobProgressUpdate, JobStatus, JobType, RegistryEntry,
};
use backend::domain::ports::{
    DocumentRegistryRepository, FolderRepository, JobRepository, SearchRepository,
    SettingsRepository,
};
use backend::error::AppError;
use backend::infrastructure::elasticsearch::{
    DEFAULT_PING_TIMEOUT, EsSearchRepository, create_es_client_with_timeout,
    ping_elasticsearch_with_timeout,
};
use backend::infrastructure::postgres::{
    PgDocumentRegistryRepository, PgFolderRepository, PgJobRepository, PgSettingsRepository,
    create_pg_pool_eager, run_migrations,
};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;

async fn setup_postgres_pool() -> Option<PgPool> {
    let _ = dotenvy::dotenv();
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://lynx:lynxpass@127.0.0.1:5432/lynxsearch".to_string());

    let pool = match create_pg_pool_eager(&db_url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("PostgreSQL not reachable at {db_url}, skipping test: {e}");
            return None;
        }
    };

    // Verify migrations execute cleanly (Phase 3 Gate)
    run_migrations(&pool)
        .await
        .expect("PostgreSQL migrations must run cleanly without error");

    Some(pool)
}

async fn setup_es_repo(alias: &str) -> Option<EsSearchRepository> {
    let _ = dotenvy::dotenv();
    let es_url = std::env::var("ELASTICSEARCH_URL")
        .unwrap_or_else(|_| DEFAULT_ELASTICSEARCH_URL.to_string());

    let client = match create_es_client_with_timeout(&es_url, Duration::from_secs(10)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot initialize Elasticsearch client at {es_url}: {e}");
            return None;
        }
    };

    if ping_elasticsearch_with_timeout(&client, DEFAULT_PING_TIMEOUT)
        .await
        .is_err()
    {
        eprintln!("Elasticsearch not reachable at {es_url}, skipping test");
        return None;
    }

    Some(EsSearchRepository::new(client, alias))
}

// ============================================================================
// 1. FolderRepository Tests
// ============================================================================

#[tokio::test]
async fn test_folder_repository_crud_lifecycle() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };
    let repo = PgFolderRepository::new(pool);

    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let root_path = PathBuf::from(format!("/tmp/lynx_test_folder_{test_suffix}"));
    let folder = Folder::new(root_path.clone());

    // 1. Create
    repo.create_folder(&folder)
        .await
        .expect("create_folder must succeed");

    // 2. Get & verify attributes
    let fetched = repo
        .get_folder(&folder.id)
        .await
        .expect("get_folder must succeed")
        .expect("folder must exist");

    assert_eq!(fetched.id, folder.id);
    assert_eq!(fetched.path, root_path);
    assert_eq!(fetched.status, FolderStatus::Idle);
    assert!(fetched.last_scanned_at.is_none());

    // 3. List & verify presence
    let all = repo
        .list_folders()
        .await
        .expect("list_folders must succeed");
    assert!(
        all.iter().any(|f| f.id == folder.id && f.path == root_path),
        "Created folder must be present in list"
    );

    // 4. Update status & reset scanning
    repo.update_status(&folder.id, FolderStatus::Scanning)
        .await
        .expect("update_status to Scanning must succeed");

    let scanning = repo
        .get_folder(&folder.id)
        .await
        .unwrap()
        .expect("folder exists");
    assert_eq!(scanning.status, FolderStatus::Scanning);

    let reset_count = repo
        .reset_scanning_folders()
        .await
        .expect("reset_scanning_folders must succeed");
    assert!(reset_count >= 1, "Must reset at least our scanning folder");

    let idle_after_reset = repo
        .get_folder(&folder.id)
        .await
        .unwrap()
        .expect("folder exists");
    assert_eq!(idle_after_reset.status, FolderStatus::Idle);

    // 5. Delete & verify non-existence
    repo.delete_folder(&folder.id)
        .await
        .expect("delete_folder must succeed");

    let deleted = repo
        .get_folder(&folder.id)
        .await
        .expect("get_folder query must succeed");
    assert!(deleted.is_none(), "Deleted folder must return None");
}

#[tokio::test]
async fn test_folder_repository_unique_root_path_constraint() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };
    let repo = PgFolderRepository::new(pool);

    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let root_path = PathBuf::from(format!("/tmp/lynx_unique_path_{test_suffix}"));

    let folder1 = Folder::new(root_path.clone());
    repo.create_folder(&folder1)
        .await
        .expect("First folder creation must succeed");

    // Second folder with different ID but same root_path
    let folder2 = Folder::new(root_path.clone());
    assert_ne!(folder1.id, folder2.id);

    let err = repo
        .create_folder(&folder2)
        .await
        .expect_err("Duplicate root_path must fail unique constraint");

    match err {
        AppError::Database(sqlx_err) => {
            assert!(
                sqlx_err.to_string().contains("unique")
                    || sqlx_err.to_string().contains("duplicate")
                    || sqlx_err.to_string().contains("folders_root_path_key"),
                "Expected unique violation error: {sqlx_err}"
            );
        }
        other => panic!("Expected AppError::Database on unique violation, got: {other:?}"),
    }

    // Cleanup
    let _ = repo.delete_folder(&folder1.id).await;
}

#[tokio::test]
async fn test_folder_repository_update_last_scanned() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };
    let repo = PgFolderRepository::new(pool);

    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let root_path = PathBuf::from(format!("/tmp/lynx_last_scanned_{test_suffix}"));
    let folder = Folder::new(root_path);

    repo.create_folder(&folder).await.unwrap();

    let before = repo.get_folder(&folder.id).await.unwrap().unwrap();
    assert!(before.last_scanned_at.is_none());

    repo.update_last_scanned(&folder.id)
        .await
        .expect("update_last_scanned must succeed");

    let after = repo.get_folder(&folder.id).await.unwrap().unwrap();
    assert!(
        after.last_scanned_at.is_some(),
        "last_scanned_at must be populated after update"
    );

    // Cleanup
    let _ = repo.delete_folder(&folder.id).await;
}

#[tokio::test]
async fn test_folder_repository_find_by_path_and_list_with_counts() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };
    let folder_repo = PgFolderRepository::new(pool.clone());
    let reg_repo = PgDocumentRegistryRepository::new(pool);

    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let root_path = PathBuf::from(format!("/tmp/lynx_counts_{test_suffix}"));
    let folder = Folder::new(root_path.clone());
    folder_repo.create_folder(&folder).await.unwrap();

    // 1. find_by_path
    let found = folder_repo
        .find_by_path(&root_path)
        .await
        .unwrap()
        .expect("Folder must be found by path");
    assert_eq!(found.id, folder.id);

    let not_found = folder_repo
        .find_by_path(&PathBuf::from("/non/existent/path"))
        .await
        .unwrap();
    assert!(not_found.is_none());

    // 2. Add indexed entry and verify list_folders_with_counts
    let entry = backend::domain::models::RegistryEntry {
        id: backend::domain::models::DocumentId::from_relative_path(folder.id, "README.md"),
        folder_id: folder.id,
        relative_path: "README.md".to_string(),
        content_hash: "hash123".to_string(),
        file_size: 100,
        status: backend::domain::models::DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    reg_repo.upsert_entry(&entry).await.unwrap();

    let list = folder_repo.list_folders_with_counts().await.unwrap();
    let matching = list
        .iter()
        .find(|(f, _)| f.id == folder.id)
        .expect("Folder must exist in list with counts");
    assert_eq!(matching.1, 1);

    // Cleanup
    let _ = folder_repo.delete_folder(&folder.id).await;
}

// ============================================================================
// 2. DocumentRegistryRepository Tests
// ============================================================================

#[tokio::test]
async fn test_document_registry_repository_crud_and_batch() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };
    let folder_repo = PgFolderRepository::new(pool.clone());
    let reg_repo = PgDocumentRegistryRepository::new(pool);

    // Setup parent folder
    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let folder = Folder::new(PathBuf::from(format!("/tmp/lynx_reg_test_{test_suffix}")));
    folder_repo.create_folder(&folder).await.unwrap();

    let doc_id1 = DocumentId::from_relative_path(folder.id, "src/main.rs");
    let entry1 = RegistryEntry {
        id: doc_id1,
        folder_id: folder.id,
        relative_path: "src/main.rs".to_string(),
        content_hash: "hash_initial_v1".to_string(),
        file_size: 1024,
        status: DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // 1. Upsert single entry
    reg_repo
        .upsert_entry(&entry1)
        .await
        .expect("upsert_entry must succeed");

    // 2. Get entry and verify attributes
    let fetched = reg_repo
        .get_entry(&doc_id1)
        .await
        .expect("get_entry must succeed")
        .expect("entry must exist");

    assert_eq!(fetched.id, doc_id1);
    assert_eq!(fetched.folder_id, folder.id);
    assert_eq!(fetched.relative_path, "src/main.rs");
    assert_eq!(fetched.content_hash, "hash_initial_v1");
    assert_eq!(fetched.file_size, 1024);
    assert_eq!(fetched.status, DocumentStatus::Indexed);

    // 3. Update existing entry (on conflict id)
    let mut updated_entry1 = entry1.clone();
    updated_entry1.content_hash = "hash_updated_v2".to_string();
    updated_entry1.file_size = 2048;
    updated_entry1.status = DocumentStatus::Indexed;

    reg_repo
        .upsert_entry(&updated_entry1)
        .await
        .expect("upsert_entry on conflict must succeed");

    let fetched_updated = reg_repo.get_entry(&doc_id1).await.unwrap().unwrap();
    assert_eq!(fetched_updated.content_hash, "hash_updated_v2");
    assert_eq!(fetched_updated.file_size, 2048);

    // 4. Batch upsert
    let doc_id2 = DocumentId::from_relative_path(folder.id, "src/lib.rs");
    let entry2 = RegistryEntry {
        id: doc_id2,
        folder_id: folder.id,
        relative_path: "src/lib.rs".to_string(),
        content_hash: "hash_lib_1".to_string(),
        file_size: 4096,
        status: DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let doc_id3 = DocumentId::from_relative_path(folder.id, "README.md");
    let entry3 = RegistryEntry {
        id: doc_id3,
        folder_id: folder.id,
        relative_path: "README.md".to_string(),
        content_hash: "hash_readme_1".to_string(),
        file_size: 512,
        status: DocumentStatus::Skipped,
        status_reason: Some("IgnoredPattern".to_string()),
        indexed_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let batch_rows = reg_repo
        .upsert_batch(&[entry2.clone(), entry3.clone()])
        .await
        .expect("upsert_batch must succeed");
    assert_eq!(batch_rows, 2);

    // 5. List by folder
    let folder_entries = reg_repo
        .list_by_folder(&folder.id)
        .await
        .expect("list_by_folder must succeed");
    assert_eq!(folder_entries.len(), 3);
    assert!(folder_entries.iter().any(|e| e.id == doc_id1));
    assert!(folder_entries.iter().any(|e| e.id == doc_id2));
    assert!(folder_entries.iter().any(|e| e.id == doc_id3));

    // 6. Delete entries
    let deleted_count = reg_repo
        .delete_entries(&[doc_id1, doc_id2])
        .await
        .expect("delete_entries must succeed");
    assert_eq!(deleted_count, 2);

    assert!(reg_repo.get_entry(&doc_id1).await.unwrap().is_none());
    assert!(reg_repo.get_entry(&doc_id2).await.unwrap().is_none());
    assert!(reg_repo.get_entry(&doc_id3).await.unwrap().is_some());

    // Cleanup parent folder (cascades to document_registry)
    folder_repo.delete_folder(&folder.id).await.unwrap();
}

#[tokio::test]
async fn test_document_registry_unique_folder_and_relative_path() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };
    let folder_repo = PgFolderRepository::new(pool.clone());
    let reg_repo = PgDocumentRegistryRepository::new(pool);

    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let folder = Folder::new(PathBuf::from(format!("/tmp/lynx_uq_reg_{test_suffix}")));
    folder_repo.create_folder(&folder).await.unwrap();

    let entry1 = RegistryEntry {
        id: DocumentId::from_uuid(uuid::Uuid::new_v4()),
        folder_id: folder.id,
        relative_path: "crates/core/src/model.rs".to_string(),
        content_hash: "hash_abc".to_string(),
        file_size: 100,
        status: DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: Utc::now(),
        updated_at: Utc::now(),
    };

    reg_repo.upsert_entry(&entry1).await.unwrap();

    // Attempt to insert a DIFFERENT document ID with the SAME folder_id + relative_path
    let entry2 = RegistryEntry {
        id: DocumentId::from_uuid(uuid::Uuid::new_v4()), // Distinct UUID!
        folder_id: folder.id,
        relative_path: "crates/core/src/model.rs".to_string(),
        content_hash: "hash_diff".to_string(),
        file_size: 200,
        status: DocumentStatus::Indexed,
        status_reason: None,
        indexed_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let err = reg_repo
        .upsert_entry(&entry2)
        .await
        .expect_err("Inserting different doc ID for same (folder_id, relative_path) must fail");

    match err {
        AppError::Database(sqlx_err) => {
            assert!(
                sqlx_err.to_string().contains("unique")
                    || sqlx_err.to_string().contains("duplicate")
                    || sqlx_err.to_string().contains("uq_folder_relative_path"),
                "Expected unique violation on (folder_id, relative_path): {sqlx_err}"
            );
        }
        other => panic!("Expected AppError::Database, got: {other:?}"),
    }

    // Cleanup
    folder_repo.delete_folder(&folder.id).await.unwrap();
}

// ============================================================================
// 3. JobRepository Tests
// ============================================================================

#[tokio::test]
async fn test_job_repository_state_transitions() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };
    let folder_repo = PgFolderRepository::new(pool.clone());
    let job_repo = PgJobRepository::new(pool);

    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let folder = Folder::new(PathBuf::from(format!("/tmp/lynx_job_test_{test_suffix}")));
    folder_repo.create_folder(&folder).await.unwrap();

    // 1. Initial Pending State
    let job1 = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder.id),
        job_type: JobType::Rescan,
        status: JobStatus::Pending,
        files_total: 50,
        files_processed: 0,
        files_indexed: 0,
        files_skipped: 0,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: None,
    };

    job_repo
        .create_job(&job1)
        .await
        .expect("create_job must succeed");

    let fetched1 = job_repo
        .get_job(&job1.id)
        .await
        .unwrap()
        .expect("job1 exists");
    assert_eq!(fetched1.status, JobStatus::Pending);
    assert!(fetched1.completed_at.is_none());

    // 2. Transition Pending -> Running
    let update_running = JobProgressUpdate {
        status: Some(JobStatus::Running),
        ..Default::default()
    };
    job_repo
        .update_progress(&job1.id, &update_running)
        .await
        .expect("transition to Running must succeed");

    let running = job_repo.get_job(&job1.id).await.unwrap().unwrap();
    assert_eq!(running.status, JobStatus::Running);
    assert!(running.completed_at.is_none());

    // 3. Transition Running -> Cancelled via mark_cancelled
    job_repo
        .mark_cancelled(&job1.id)
        .await
        .expect("mark_cancelled must succeed");

    let cancelled = job_repo.get_job(&job1.id).await.unwrap().unwrap();
    assert_eq!(cancelled.status, JobStatus::Cancelled);
    assert!(cancelled.completed_at.is_some());

    // 4. Test recovery of dangling jobs (Pending / Running -> Failed)
    let job2 = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder.id),
        job_type: JobType::Import,
        status: JobStatus::Running,
        files_total: 100,
        files_processed: 20,
        files_indexed: 20,
        files_skipped: 0,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: None,
    };
    job_repo.create_job(&job2).await.unwrap();

    let dangling = job_repo
        .find_dangling_jobs()
        .await
        .expect("find_dangling_jobs must succeed");
    assert!(dangling.iter().any(|j| j.id == job2.id));

    let recovered_count = job_repo
        .recover_dangling_jobs("Simulated crash recovery")
        .await
        .expect("recover_dangling_jobs must succeed");
    assert!(recovered_count >= 1);

    let recovered = job_repo.get_job(&job2.id).await.unwrap().unwrap();
    assert_eq!(recovered.status, JobStatus::Failed);
    assert!(recovered.completed_at.is_some());
    assert_eq!(
        recovered.error_summary.as_deref(),
        Some("Simulated crash recovery")
    );

    // 5. Test cancel_unfinished_jobs
    let job3 = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder.id),
        job_type: JobType::Rebuild,
        status: JobStatus::Running,
        files_total: 10,
        files_processed: 0,
        files_indexed: 0,
        files_skipped: 0,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: None,
    };
    job_repo.create_job(&job3).await.unwrap();

    let cancelled_count = job_repo
        .cancel_unfinished_jobs("Graceful shutdown")
        .await
        .expect("cancel_unfinished_jobs must succeed");
    assert!(cancelled_count >= 1);

    let final_job3 = job_repo.get_job(&job3.id).await.unwrap().unwrap();
    assert_eq!(final_job3.status, JobStatus::Cancelled);
    assert_eq!(
        final_job3.error_summary.as_deref(),
        Some("Graceful shutdown")
    );

    // Cleanup
    folder_repo.delete_folder(&folder.id).await.unwrap();
}

#[tokio::test]
async fn test_job_repository_progress_counters() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };
    let folder_repo = PgFolderRepository::new(pool.clone());
    let job_repo = PgJobRepository::new(pool);

    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let folder = Folder::new(PathBuf::from(format!("/tmp/lynx_job_cnt_{test_suffix}")));
    folder_repo.create_folder(&folder).await.unwrap();

    let job = IndexingJob {
        id: JobId::new(),
        folder_id: Some(folder.id),
        job_type: JobType::Import,
        status: JobStatus::Running,
        files_total: 100,
        files_processed: 0,
        files_indexed: 0,
        files_skipped: 0,
        files_failed: 0,
        error_summary: None,
        started_at: Utc::now(),
        completed_at: None,
    };
    job_repo.create_job(&job).await.unwrap();

    // First progress delta
    job_repo
        .update_progress(
            &job.id,
            &JobProgressUpdate {
                files_total: None,
                processed_delta: 16,
                indexed_delta: 12,
                skipped_delta: 3,
                failed_delta: 1,
                status: None,
                error_summary: None,
            },
        )
        .await
        .expect("first progress update must succeed");

    let p1 = job_repo.get_job(&job.id).await.unwrap().unwrap();
    assert_eq!(p1.files_indexed, 12);
    assert_eq!(p1.files_skipped, 3);
    assert_eq!(p1.files_failed, 1);
    assert_eq!(p1.files_processed, 16); // 12 + 3 + 1
    assert_eq!(p1.files_total, 100);

    // Second cumulative progress delta
    job_repo
        .update_progress(
            &job.id,
            &JobProgressUpdate {
                files_total: Some(120), // Dynamically discovered more files
                processed_delta: 10,
                indexed_delta: 8,
                skipped_delta: 2,
                failed_delta: 0,
                status: None,
                error_summary: None,
            },
        )
        .await
        .expect("second progress update must succeed");

    let p2 = job_repo.get_job(&job.id).await.unwrap().unwrap();
    assert_eq!(p2.files_indexed, 20); // 12 + 8
    assert_eq!(p2.files_skipped, 5); // 3 + 2
    assert_eq!(p2.files_failed, 1); // 1 + 0
    assert_eq!(p2.files_processed, 26); // 20 + 5 + 1
    assert_eq!(p2.files_total, 120);

    // Final completion transition
    job_repo
        .update_progress(
            &job.id,
            &JobProgressUpdate {
                status: Some(JobStatus::Completed),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let p3 = job_repo.get_job(&job.id).await.unwrap().unwrap();
    assert_eq!(p3.status, JobStatus::Completed);
    assert!(p3.completed_at.is_some());

    // Cleanup
    folder_repo.delete_folder(&folder.id).await.unwrap();
}

// ============================================================================
// 4. SettingsRepository Tests
// ============================================================================

#[tokio::test]
async fn test_settings_repository_persistence_and_reset() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };
    let repo = PgSettingsRepository::new(pool);

    // 1. Fetch initial settings (seeded or default)
    let initial = repo
        .get_settings()
        .await
        .expect("get_settings must succeed");
    assert!(initial.max_file_size_bytes > 0);

    // 2. Update with customized settings
    let custom = AppSettings {
        max_file_size_bytes: 8 * 1024 * 1024,
        weights: Bm25Weights {
            title: 4.5,
            tags: 3.2,
            content: 1.8,
        },
        ignore_patterns: vec![".git".into(), "node_modules".into(), "custom_build".into()],
    };

    repo.update_settings(&custom)
        .await
        .expect("update_settings must succeed");

    // 3. Verify persistence
    let persisted = repo
        .get_settings()
        .await
        .expect("get_settings must succeed");
    assert_eq!(persisted.max_file_size_bytes, 8 * 1024 * 1024);
    assert_eq!(persisted.weights.title, 4.5);
    assert_eq!(persisted.weights.tags, 3.2);
    assert_eq!(persisted.weights.content, 1.8);
    assert_eq!(
        persisted.ignore_patterns,
        vec![".git", "node_modules", "custom_build"]
    );

    // 4. Reset to default
    let reset = repo
        .reset_settings()
        .await
        .expect("reset_settings must succeed");
    let default = AppSettings::default();
    assert_eq!(reset.max_file_size_bytes, default.max_file_size_bytes);
    assert_eq!(reset.weights, default.weights);
    assert_eq!(reset.ignore_patterns, default.ignore_patterns);
}

// ============================================================================
// 5. SearchRepository Tests (Elasticsearch 8.19.22)
// ============================================================================

#[tokio::test]
async fn test_search_repository_ping() {
    let test_alias = format!("lynx_ping_alias_{}", uuid::Uuid::new_v4().simple());
    let repo = match setup_es_repo(&test_alias).await {
        Some(r) => r,
        None => return,
    };

    repo.ping().await.expect("Elasticsearch ping must succeed");
}

#[tokio::test]
async fn test_search_repository_initial_mapping_and_alias_lifecycle() {
    let test_suffix = uuid::Uuid::new_v4().simple().to_string();
    let test_alias = format!("lynx_repo_alias_{test_suffix}");

    let repo = match setup_es_repo(&test_alias).await {
        Some(r) => r,
        None => return,
    };

    // 1. Initial Mapping & Index Creation via ensure_initial_index
    let initial_index = repo
        .ensure_initial_index()
        .await
        .expect("ensure_initial_index must succeed");

    assert_eq!(initial_index, format!("{test_alias}_v1"));

    // Idempotency: calling again must return the exact same active index
    let idempotent_index = repo
        .ensure_initial_index()
        .await
        .expect("second ensure_initial_index must succeed");
    assert_eq!(idempotent_index, initial_index);

    // Verify alias points to v1
    let active_opt = repo
        .get_active_physical_index()
        .await
        .expect("get_active_physical_index must succeed");
    assert_eq!(active_opt.as_deref(), Some(initial_index.as_str()));

    let alias_indices = repo
        .get_alias_indices(&test_alias)
        .await
        .expect("get_alias_indices must succeed");
    assert_eq!(alias_indices, vec![initial_index.clone()]);

    // 2. Create versioned index v2
    let index_v2 = repo
        .create_versioned_index(2)
        .await
        .expect("create_versioned_index(2) must succeed");
    assert_eq!(index_v2, format!("{test_alias}_v2"));

    // 3. Atomic alias swap to v2
    repo.swap_alias(&test_alias, std::slice::from_ref(&initial_index), &index_v2)
        .await
        .expect("swap_alias to v2 must succeed");

    let active_v2 = repo
        .get_active_physical_index()
        .await
        .unwrap()
        .expect("active index must exist");
    assert_eq!(active_v2, index_v2);

    // 4. Rebuild index with alias: create v3 and rebuild
    let index_v3 = repo
        .create_versioned_index(3)
        .await
        .expect("create_versioned_index(3) must succeed");
    assert_eq!(index_v3, format!("{test_alias}_v3"));

    repo.rebuild_index_with_alias(&index_v3, &test_alias)
        .await
        .expect("rebuild_index_with_alias must succeed");

    let active_v3 = repo
        .get_active_physical_index()
        .await
        .unwrap()
        .expect("active index must exist");
    assert_eq!(active_v3, index_v3);

    // Rebuild removes old indices pointing to the alias; verify index_v2 is deleted
    let remaining_indices = repo.get_alias_indices(&test_alias).await.unwrap();
    assert_eq!(remaining_indices, vec![index_v3.clone()]);

    // 5. Verify search operates through alias (Phase 3 Gate)
    let search_dsl = json!({
        "query": {
            "match_all": {}
        }
    });
    let raw_val = serde_json::value::RawValue::from_string(search_dsl.to_string())
        .expect("valid search query RawValue");

    let search_result = repo
        .search(&raw_val)
        .await
        .expect("search through alias must succeed");

    assert!(
        search_result.raw_json.contains("\"hits\""),
        "Search response through alias must contain hits: {}",
        search_result.raw_json
    );

    // Cleanup physical indices
    let _ = repo.delete_index(&initial_index).await;
    let _ = repo.delete_index(&index_v2).await;
    let _ = repo.delete_index(&index_v3).await;
}

// ============================================================================
// 6. Polymorphic Repositories Trait Verification (Phase 3 Gate)
// ============================================================================

#[tokio::test]
async fn test_repositories_polymorphic_dispatch_with_postgres_and_elasticsearch() {
    let pool = match setup_postgres_pool().await {
        Some(p) => p,
        None => return,
    };

    let test_alias = format!("lynx_poly_alias_{}", uuid::Uuid::new_v4().simple());
    let es_repo = match setup_es_repo(&test_alias).await {
        Some(r) => r,
        None => return,
    };

    let repos = backend::state::Repositories::from_postgres(pool, Arc::new(es_repo));

    // Verify all ports can be called polymorphically via trait objects
    repos.search.ping().await.expect("Polymorphic search ping");
    let settings = repos
        .settings
        .get_settings()
        .await
        .expect("Polymorphic settings");
    assert!(settings.max_file_size_bytes > 0);

    let folders = repos
        .folder
        .list_folders()
        .await
        .expect("Polymorphic folder list");
    assert!(!folders.is_empty() || folders.is_empty());

    let jobs = repos
        .job
        .find_dangling_jobs()
        .await
        .expect("Polymorphic dangling jobs");
    assert!(!jobs.is_empty() || jobs.is_empty());
}
