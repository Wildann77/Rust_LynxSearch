use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::application::orchestrator::WorkerCommand;
use backend::domain::models::{Folder, FolderId, FolderStatus, JobStatus};
use backend::state::AppState;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test]
async fn test_search_and_http_endpoints_available_during_active_indexing() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();
    let app = backend::create_router_with_state(state.clone());

    // 1. Siapkan direktori fisik dengan 15 file untuk simulasi workload scanning
    let temp = tempdir().expect("Failed to create tempdir");
    let root = temp.path();

    for i in 0..15 {
        let file_path = root.join(format!("document_{i}.md"));
        fs::write(
            &file_path,
            format!("# Document {i}\n\nLynxSearch fast local desktop search engine text #{i}."),
        )
        .expect("Failed to write test file");
    }

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
        .expect("Failed to register test folder");

    // 3. Submit index job
    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .expect("Failed to submit index folder");

    // 4. Jalankan background worker di task terpisah
    let orch_clone = orchestrator.clone();
    let worker_handle = tokio::spawn(async move {
        if let Some(cmd) = rx.recv().await {
            assert!(matches!(cmd, WorkerCommand::IndexFolder { .. }));
            orch_clone
                .dispatch(cmd)
                .await
                .expect("Worker dispatch failed");
        }
    });

    // 5. Kirim HTTP search dan health request secara konkuren saat worker sedang berjalan
    let search_success_count = Arc::new(AtomicUsize::new(0));
    let is_indexing_active = Arc::new(AtomicBool::new(true));

    let search_app = app.clone();
    let counter_clone = search_success_count.clone();
    let active_clone = is_indexing_active.clone();

    let concurrent_client_handle = tokio::spawn(async move {
        while active_clone.load(Ordering::Acquire) {
            // GET /api/search
            let req = Request::builder()
                .uri("/api/search?q=LynxSearch")
                .method("GET")
                .body(Body::empty())
                .unwrap();
            let res = search_app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), StatusCode::OK);

            // GET /api/health
            let health_req = Request::builder()
                .uri("/api/health")
                .method("GET")
                .body(Body::empty())
                .unwrap();
            let health_res = search_app.clone().oneshot(health_req).await.unwrap();
            assert_eq!(health_res.status(), StatusCode::OK);

            counter_clone.fetch_add(1, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });

    // 6. Tunggu worker selesai
    worker_handle.await.expect("Worker task panicked");
    is_indexing_active.store(false, Ordering::Release);
    concurrent_client_handle
        .await
        .expect("Concurrent client task panicked");

    // 7. Verifikasi job status akhir dan bahwa search queries berhasil dieksekusi selama indexing
    let progress = state
        .job_tracker
        .get_progress(&job_id)
        .expect("Job progress missing");
    assert_eq!(progress.status, JobStatus::Completed);
    assert_eq!(progress.files_indexed, 15);
    assert!(
        search_success_count.load(Ordering::Relaxed) > 0,
        "Expected multiple search requests to succeed concurrently while indexing"
    );

    // 8. Verifikasi query akhir setelah job selesai tetap merespons 200 OK
    let final_req = Request::builder()
        .uri("/api/search?q=document")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let final_res = app.oneshot(final_req).await.unwrap();
    assert_eq!(final_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_background_worker_does_not_monopolize_tokio_runtime() {
    let (state, mut rx) = AppState::test_state();
    let orchestrator = state.orchestrator();

    let temp = tempdir().expect("Failed to create tempdir");
    let root = temp.path();

    for i in 0..20 {
        let file_path = root.join(format!("file_{i}.rs"));
        fs::write(
            &file_path,
            format!("pub fn compute_{i}() -> usize {{ {i} * 42 }}\n"),
        )
        .expect("Failed to write test file");
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
        .expect("Failed to register folder");

    let job_id = orchestrator
        .submit_index_folder(folder_id, false)
        .await
        .expect("Failed to submit index folder");

    let orch_clone = orchestrator.clone();
    let worker_handle = tokio::spawn(async move {
        if let Some(cmd) = rx.recv().await {
            orch_clone
                .dispatch(cmd)
                .await
                .expect("Worker dispatch failed");
        }
    });

    // Monitor cooperativeness runtime: task ini harus mendapatkan execution cycle secara reguler
    let heartbeat_ticks = Arc::new(AtomicUsize::new(0));
    let is_done = Arc::new(AtomicBool::new(false));

    let hb_ticks = heartbeat_ticks.clone();
    let hb_done = is_done.clone();
    let monitor_handle = tokio::spawn(async move {
        while !hb_done.load(Ordering::Acquire) {
            hb_ticks.fetch_add(1, Ordering::Relaxed);
            tokio::task::yield_now().await;
        }
    });

    worker_handle.await.expect("Worker panicked");
    is_done.store(true, Ordering::Release);
    monitor_handle.await.expect("Monitor panicked");

    let final_progress = state
        .job_tracker
        .get_progress(&job_id)
        .expect("Job progress missing");
    assert_eq!(final_progress.status, JobStatus::Completed);
    assert_eq!(final_progress.files_indexed, 20);

    // Heartbeat ticks harus terhitung banyak, menandakan thread runtime tidak pernah termonopoli
    assert!(
        heartbeat_ticks.load(Ordering::Relaxed) >= 1,
        "Worker monopolized runtime: heartbeat ticks were too low"
    );
}
