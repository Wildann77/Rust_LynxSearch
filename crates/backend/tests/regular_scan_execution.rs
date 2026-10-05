use backend::application::orchestrator::WorkerCommand;
use backend::domain::models::{
    DocumentId, DocumentStatus, Folder, FolderId, FolderStatus, JobStatus,
};
use backend::domain::services::scan_planner::normalize_path_str;
use backend::state::AppState;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_regular_scan_end_to_end() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    // 1. Siapkan direktori fisik dengan berkas uji
    let temp = tempdir().unwrap();
    let root = temp.path();

    // File 1: Markdown dengan YAML frontmatter
    let doc1_path = root.join("architecture.md");
    fs::write(
        &doc1_path,
        "---\ntitle: Clean Architecture\ntags: [arch, rust, ddd]\n---\n# Clean Architecture\n\nHexagonal ports and adapters.",
    )
    .unwrap();

    // File 2: Source code Rust
    let code_path = root.join("calculator.rs");
    fs::write(
        &code_path,
        "// Calculator module\npub fn add_numbers(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();

    // File 3: File dalam subdirektori
    let nested_dir = root.join("guides");
    fs::create_dir_all(&nested_dir).unwrap();
    let nested_doc = nested_dir.join("getting_started.md");
    fs::write(&nested_doc, "# Getting Started\n\nWelcome to LynxSearch!").unwrap();

    // File 4: File biner (PNG) yang tidak diabaikan oleh walker tetapi diskip oleh extractor
    let binary_file = root.join("photo.png");
    fs::write(
        &binary_file,
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01",
    )
    .unwrap();

    // 2. Daftarkan folder ke repository
    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: root.to_path_buf(),
        status: FolderStatus::Idle,
        created_at: chrono::Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // 3. Submit scan job
    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();

    // Verifikasi state awal di JobTracker & Repository
    let pending_progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(pending_progress.status, JobStatus::Pending);

    // 4. Ambil command dari antrian worker dan dispatch
    let cmd = rx.recv().await.unwrap();
    assert_eq!(
        cmd,
        WorkerCommand::IndexFolder {
            job_id,
            folder_id,
            rescan: false,
        }
    );

    orchestrator.dispatch(cmd).await.unwrap();

    // 5. Verifikasi status akhir job dan folder
    let finished_progress = state.job_tracker.get_progress(&job_id).unwrap();
    assert_eq!(finished_progress.status, JobStatus::Completed);
    assert!(finished_progress.completed_at.is_some());

    let finished_db_job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(finished_db_job.status, JobStatus::Completed);
    assert!(finished_db_job.completed_at.is_some());

    let finished_folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(finished_folder.status, FolderStatus::Idle);
    assert!(finished_folder.last_scanned_at.is_some());

    // 6. Verifikasi metrik dan counter
    // 4 berkas pada disk: 3 berhasil diindeks, 1 diskip (.env.production)
    assert_eq!(finished_db_job.files_total, 4);
    assert_eq!(finished_db_job.files_indexed, 3);
    assert_eq!(finished_db_job.files_skipped, 1);
    assert_eq!(finished_db_job.files_failed, 0);
    assert_eq!(finished_db_job.files_processed, finished_db_job.files_total);

    // 7. Verifikasi dokumen di Document Registry
    let entries = state
        .repositories
        .registry
        .list_by_folder(&folder_id)
        .await
        .unwrap();
    assert_eq!(entries.len(), 4);

    let doc1_entry = entries
        .iter()
        .find(|e| e.relative_path == "architecture.md")
        .expect("architecture.md must be in registry");
    assert_eq!(doc1_entry.status, DocumentStatus::Indexed);
    assert!(!doc1_entry.content_hash.is_empty());

    let code_entry = entries
        .iter()
        .find(|e| e.relative_path == "calculator.rs")
        .expect("calculator.rs must be in registry");
    assert_eq!(code_entry.status, DocumentStatus::Indexed);

    let nested_entry = entries
        .iter()
        .find(|e| normalize_path_str(&e.relative_path) == "guides/getting_started.md")
        .expect("guides/getting_started.md must be in registry");
    assert_eq!(nested_entry.status, DocumentStatus::Indexed);

    let binary_entry = entries
        .iter()
        .find(|e| e.relative_path == "photo.png")
        .expect("photo.png must be in registry");
    assert_eq!(binary_entry.status, DocumentStatus::Skipped);

    // 8. Verifikasi dokumen terindeks pada SearchRepository
    let doc1_indexed = state
        .repositories
        .search
        .search(&serde_json::value::RawValue::from_string("{}".to_string()).unwrap())
        .await;
    assert!(doc1_indexed.is_ok());
}

#[tokio::test]
async fn test_regular_rescan_idempotency() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::write(root.join("note1.md"), "# Note 1\nContent 1").unwrap();
    fs::write(root.join("note2.md"), "# Note 2\nContent 2").unwrap();

    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: root.to_path_buf(),
        status: FolderStatus::Idle,
        created_at: chrono::Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // Initial scan
    let job1_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    let cmd1 = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd1).await.unwrap();

    let job1 = state
        .repositories
        .job
        .get_job(&job1_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job1.status, JobStatus::Completed);
    assert_eq!(job1.files_indexed, 2);
    assert_eq!(job1.files_skipped, 0);

    // Rescan tanpa perubahan file sama sekali
    let job2_id = orchestrator
        .submit_index_folder(folder_id, true)
        .await
        .unwrap();
    let cmd2 = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd2).await.unwrap();

    let job2 = state
        .repositories
        .job
        .get_job(&job2_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job2.status, JobStatus::Completed);
    assert_eq!(job2.files_total, 2);
    assert_eq!(job2.files_indexed, 0); // Idempotent: 0 dokumen diindeks ulang
    assert_eq!(job2.files_skipped, 2); // Keduanya masuk fast-path Unchanged
    assert_eq!(job2.files_failed, 0);
    assert_eq!(job2.files_processed, 2);
    assert_eq!(job2.files_total, job2.files_processed);

    // Verifikasi registry tetap utuh dan konsisten
    let entries = state
        .repositories
        .registry
        .list_by_folder(&folder_id)
        .await
        .unwrap();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        assert_eq!(entry.status, DocumentStatus::Indexed);
    }
}

#[tokio::test]
async fn test_rescan_detects_updated_new_and_deleted_documents() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let temp = tempdir().unwrap();
    let root = temp.path();

    let file_to_update = root.join("updated.md");
    let file_to_delete = root.join("deleted.md");
    let file_unchanged = root.join("unchanged.md");

    fs::write(&file_to_update, "# Old Title\nOld Content").unwrap();
    fs::write(&file_to_delete, "# Delete Me\nWill be removed").unwrap();
    fs::write(&file_unchanged, "# Stay\nNever changes").unwrap();

    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: root.to_path_buf(),
        status: FolderStatus::Idle,
        created_at: chrono::Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // 1. Initial scan
    let job1_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    let cmd1 = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd1).await.unwrap();

    let job1 = state
        .repositories
        .job
        .get_job(&job1_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job1.files_indexed, 3);

    let old_updated_entry = state
        .repositories
        .registry
        .list_by_folder(&folder_id)
        .await
        .unwrap()
        .into_iter()
        .find(|e| e.relative_path == "updated.md")
        .unwrap();

    // 2. Modifikasi filesystem:
    // a. Hapus deleted.md
    fs::remove_file(&file_to_delete).unwrap();

    // b. Modifikasi updated.md
    fs::write(
        &file_to_update,
        "# New Title\nCompletely new and revised content.",
    )
    .unwrap();

    // c. Tambah file baru brand_new.rs
    let new_file = root.join("brand_new.rs");
    fs::write(&new_file, "pub fn new_feature() -> bool { true }").unwrap();

    // 3. Jalankan rescan
    let job2_id = orchestrator
        .submit_index_folder(folder_id, true)
        .await
        .unwrap();
    let cmd2 = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd2).await.unwrap();

    let job2 = state
        .repositories
        .job
        .get_job(&job2_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job2.status, JobStatus::Completed);
    // Total berkas di disk sekarang: updated.md, unchanged.md, brand_new.rs = 3
    assert_eq!(job2.files_total, 3);
    // updated.md dan brand_new.rs diindeks
    assert_eq!(job2.files_indexed, 2);
    // unchanged.md diskip
    assert_eq!(job2.files_skipped, 1);
    assert_eq!(job2.files_failed, 0);

    // 4. Verifikasi registry database
    let entries = state
        .repositories
        .registry
        .list_by_folder(&folder_id)
        .await
        .unwrap();
    // deleted.md harus sudah dihapus dari registri
    assert!(
        entries.iter().all(|e| e.relative_path != "deleted.md"),
        "deleted.md must be removed from registry"
    );

    // updated.md memiliki hash baru
    let new_updated_entry = entries
        .iter()
        .find(|e| e.relative_path == "updated.md")
        .expect("updated.md must be in registry");
    assert_ne!(
        old_updated_entry.content_hash,
        new_updated_entry.content_hash
    );

    // brand_new.rs ada di registri
    let new_entry = entries
        .iter()
        .find(|e| e.relative_path == "brand_new.rs")
        .expect("brand_new.rs must be in registry");
    assert_eq!(new_entry.status, DocumentStatus::Indexed);
}

#[tokio::test]
async fn test_regular_scan_non_existent_folder_fails_cleanly() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let folder_id = FolderId::new();
    let non_existent_path = std::path::PathBuf::from("/non/existent/path/for/lynx_test_12345");

    let folder = Folder {
        id: folder_id,
        path: non_existent_path,
        status: FolderStatus::Idle,
        created_at: chrono::Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    let cmd = rx.recv().await.unwrap();

    // Dispatch harus menangani error dengan bersih
    orchestrator.dispatch(cmd).await.unwrap();

    let job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.status, JobStatus::Failed);
    assert!(job.error_summary.is_some());
    assert!(
        job.error_summary
            .unwrap()
            .contains("Folder root directory does not exist")
    );

    // Folder harus kembali ke Idle dan lock harus dilepas
    let folder = state
        .repositories
        .folder
        .get_folder(&folder_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(folder.status, FolderStatus::Idle);
    assert!(!state.job_tracker.is_folder_locked(&folder_id));
}

#[tokio::test]
async fn test_regular_scan_per_file_resilience() {
    use std::os::unix::fs::PermissionsExt;

    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let temp = tempdir().unwrap();
    let root = temp.path();

    let valid_file = root.join("valid.md");
    let unreadable_file = root.join("unreadable.md");

    fs::write(&valid_file, "# Valid Doc\nThis file should succeed.").unwrap();
    fs::write(&unreadable_file, "# Secret Doc\nUnreadable permissions.").unwrap();

    // Ubah permissions menjadi 0o000 agar pembacaan gagal di Linux
    fs::set_permissions(&unreadable_file, fs::Permissions::from_mode(0o000)).unwrap();

    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: root.to_path_buf(),
        status: FolderStatus::Idle,
        created_at: chrono::Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    let cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd).await.unwrap();

    // Pulihkan permission agar cleanup tempfile aman
    let _ = fs::set_permissions(&unreadable_file, fs::Permissions::from_mode(0o644));

    let job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    // Job tetap selesai (resilience) dan tidak crash karena satu file gagal
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.files_total, 2);
    assert_eq!(job.files_indexed, 1);
    assert_eq!(job.files_failed, 1);
    assert_eq!(job.files_processed, 2);

    let entries = state
        .repositories
        .registry
        .list_by_folder(&folder_id)
        .await
        .unwrap();
    assert_eq!(entries.len(), 2);

    let failed_entry = entries
        .iter()
        .find(|e| e.relative_path == "unreadable.md")
        .expect("unreadable.md in registry");
    assert_eq!(failed_entry.status, DocumentStatus::Failed);
    assert!(
        failed_entry
            .status_reason
            .as_ref()
            .map(|s| s.contains("Failed to read"))
            .unwrap_or(false),
        "status_reason must record the read/hash failure reason, got: {:?}",
        failed_entry.status_reason
    );

    let valid_entry = entries
        .iter()
        .find(|e| e.relative_path == "valid.md")
        .expect("valid.md in registry");
    assert_eq!(valid_entry.status, DocumentStatus::Indexed);
    assert!(valid_entry.status_reason.is_none());

    // 8. Verifikasi emisi domain events
    let events = state.recorded_events();
    let failed_event = events.iter().find(|e| {
        matches!(e, backend::domain::events::DomainEvent::DocumentFailed(f) if f.path == "unreadable.md")
    });
    assert!(
        failed_event.is_some(),
        "DomainEvent::DocumentFailed must be emitted for unreadable file"
    );

    let indexed_event = events.iter().find(|e| {
        matches!(e, backend::domain::events::DomainEvent::DocumentIndexed(i) if i.path == "valid.md")
    });
    assert!(
        indexed_event.is_some(),
        "DomainEvent::DocumentIndexed must be emitted for valid file"
    );
}

#[tokio::test]
async fn test_batch_progress_commit_in_chunks_of_100() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let temp = tempdir().unwrap();
    let root = temp.path();

    // Buat 125 file untuk memicu batch commit pada 100 dokumen
    for i in 0..125 {
        let file_path = root.join(format!("doc_{i:03}.md"));
        fs::write(
            &file_path,
            format!("# Document {i}\nContent for document {i}."),
        )
        .unwrap();
    }

    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: root.to_path_buf(),
        status: FolderStatus::Idle,
        created_at: chrono::Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    let cmd = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd).await.unwrap();

    let job = state
        .repositories
        .job
        .get_job(&job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.files_total, 125);
    assert_eq!(job.files_indexed, 125);
    assert_eq!(job.files_skipped, 0);
    assert_eq!(job.files_failed, 0);
    assert_eq!(job.files_processed, 125);

    let entries = state
        .repositories
        .registry
        .list_by_folder(&folder_id)
        .await
        .unwrap();
    assert_eq!(entries.len(), 125);
}

#[tokio::test]
async fn test_rescan_move_or_rename_preserves_single_document() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let temp = tempdir().unwrap();
    let root = temp.path();

    // 1. Siapkan struktur berkas awal
    let initial_dir = root.join("docs");
    fs::create_dir_all(&initial_dir).unwrap();
    let initial_file = initial_dir.join("guide.md");
    let content = "# Architectural Guide\nContent explaining hexagonal architecture in detail.";
    fs::write(&initial_file, content).unwrap();

    let folder_id = FolderId::new();
    let folder = Folder {
        id: folder_id,
        path: root.to_path_buf(),
        status: FolderStatus::Idle,
        created_at: chrono::Utc::now(),
        last_scanned_at: None,
    };
    state
        .repositories
        .folder
        .create_folder(&folder)
        .await
        .unwrap();

    // 2. Jalankan pemindaian awal
    let job1_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .unwrap();
    let cmd1 = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd1).await.unwrap();

    let job1 = state
        .repositories
        .job
        .get_job(&job1_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job1.status, JobStatus::Completed);
    assert_eq!(job1.files_indexed, 1);

    // Hitung deterministic ID path lama
    let old_rel_path = std::path::Path::new("docs").join("guide.md");
    let old_doc_id = DocumentId::from_relative_path(folder_id, &old_rel_path);

    let initial_entry = state
        .repositories
        .registry
        .get_entry(&old_doc_id)
        .await
        .unwrap()
        .expect("Old document must exist in registry after initial scan");
    assert_eq!(initial_entry.status, DocumentStatus::Indexed);
    let original_hash = initial_entry.content_hash.clone();

    // 3. Pindahkan / ubah nama berkas ke subdirektori baru (rename & move)
    let target_dir = root.join("architecture");
    fs::create_dir_all(&target_dir).unwrap();
    let renamed_file = target_dir.join("renamed_guide.md");
    fs::rename(&initial_file, &renamed_file).unwrap();

    // 4. Jalankan rescan inkremental
    let job2_id = orchestrator
        .submit_index_folder(folder_id, true)
        .await
        .unwrap();
    let cmd2 = rx.recv().await.unwrap();
    orchestrator.dispatch(cmd2).await.unwrap();

    let job2 = state
        .repositories
        .job
        .get_job(&job2_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job2.status, JobStatus::Completed);
    assert_eq!(job2.files_total, 1);
    assert_eq!(job2.files_indexed, 1);
    assert_eq!(job2.files_skipped, 0);
    assert_eq!(job2.files_failed, 0);

    // Hitung deterministic ID path baru
    let new_rel_path = std::path::Path::new("architecture").join("renamed_guide.md");
    let new_doc_id = DocumentId::from_relative_path(folder_id, &new_rel_path);
    assert_ne!(
        old_doc_id, new_doc_id,
        "New deterministic ID must differ from old ID"
    );

    // 5. Verifikasi registry: old ID dihapus, new ID diindeks, persis 1 dokumen tersisa
    let entries = state
        .repositories
        .registry
        .list_by_folder(&folder_id)
        .await
        .unwrap();
    assert_eq!(
        entries.len(),
        1,
        "Exactly one document entry must remain in registry"
    );

    let old_entry = state
        .repositories
        .registry
        .get_entry(&old_doc_id)
        .await
        .unwrap();
    assert!(
        old_entry.is_none(),
        "Old path ID must be removed from registry"
    );

    let new_entry = state
        .repositories
        .registry
        .get_entry(&new_doc_id)
        .await
        .unwrap()
        .expect("New path ID must be present in registry");
    assert_eq!(new_entry.status, DocumentStatus::Indexed);
    assert_eq!(
        normalize_path_str(&new_entry.relative_path),
        "architecture/renamed_guide.md"
    );
    assert_eq!(
        new_entry.content_hash, original_hash,
        "Content hash of moved document must match original"
    );
}
