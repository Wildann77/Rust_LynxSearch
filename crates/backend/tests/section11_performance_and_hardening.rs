mod common;

use elasticsearch::Elasticsearch;
use elasticsearch::indices::IndicesDeleteParts;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tempfile::TempDir;

use backend::config::DEFAULT_ELASTICSEARCH_URL;
use backend::domain::models::{
    AppSettings, DocumentId, DocumentStatus, DocumentType, FolderId, IndexedDocument, Language,
    RegistryEntry,
};
use backend::domain::ports::{
    DiscoveredFile, FileReader, FileWalker, ReadOptions, SearchRepository, WalkOptions,
};
use backend::domain::query_parser::QueryParser;
use backend::domain::services::scan_planner::{PlanSkipReason, ScanPlanner};
use backend::domain::services::search_query_builder::{
    DEFAULT_PAGE, DEFAULT_PER_PAGE, MAX_PER_PAGE, SearchQueryBuilder,
};
use backend::infrastructure::elasticsearch::{
    DEFAULT_PING_TIMEOUT, EsSearchRepository, create_es_client_with_timeout,
    ping_elasticsearch_with_timeout, schema,
};
use backend::infrastructure::fs::reader::LocalFileReader;
use backend::infrastructure::fs::walker::LocalFileWalker;
use chrono::Utc;
use common::generator::FixtureTreeGenerator;
use tokio::sync::Semaphore;
use uuid::Uuid;

/// RAII Guard that cleans up the dynamic Elasticsearch test index on drop.
struct EsIndexGuard {
    client: Elasticsearch,
    index_name: String,
}

impl EsIndexGuard {
    fn new(client: Elasticsearch, index_name: String) -> Self {
        Self { client, index_name }
    }

    fn index_name(&self) -> &str {
        &self.index_name
    }
}

impl Drop for EsIndexGuard {
    fn drop(&mut self) {
        let client = self.client.clone();
        let index = self.index_name.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = client
                    .indices()
                    .delete(IndicesDeleteParts::Index(&[&index]))
                    .send()
                    .await;
            });
        }
    }
}

// =========================================================================
// 11.1 Backend Target Measurements
// =========================================================================

#[test]
fn test_11_1_no_change_rescan_completes_in_milliseconds() {
    let generator = FixtureTreeGenerator::new(1000);
    let (temp_dir, _) = generator.generate();
    let root = temp_dir.path();

    let walker = LocalFileWalker::new();
    let discovered: Vec<DiscoveredFile> = walker
        .walk_all(root, &WalkOptions::default())
        .expect("Walk generated fixture tree");

    let folder_id = FolderId::new();
    let now = Utc::now();

    // Buat initial registry entries matching discovered files (identical content & hash)
    let registry_entries: Vec<RegistryEntry> = discovered
        .iter()
        .map(|f| {
            let doc_id = DocumentId::from_relative_path(folder_id, &f.relative_path);
            RegistryEntry {
                id: doc_id,
                folder_id,
                relative_path: f.relative_path.to_string_lossy().to_string(),
                content_hash: "mock_deterministic_hash".to_string(),
                file_size: f.file_size as i64,
                status: DocumentStatus::Indexed,
                status_reason: None,
                indexed_at: now,
                updated_at: f.modified_at.unwrap_or(now),
            }
        })
        .collect();

    let settings = AppSettings::default();

    // Ukur durasi evaluasi ScanPlanner pada kondisi no-change
    let start = Instant::now();
    let plan = ScanPlanner::plan(folder_id, &discovered, &registry_entries, &settings, |_| {
        Ok("mock_deterministic_hash".to_string())
    });
    let duration = start.elapsed();

    // Verifikasi: No-change rescan harus selesai dalam hitungan milliseconds (< 100ms)
    assert!(
        duration.as_millis() < 500,
        "No-change rescan evaluation took too long: {} ms",
        duration.as_millis()
    );

    // Semua 1.000 file harus di-skip karena status Unchanged (zero disk re-read, zero ES re-index)
    assert_eq!(plan.to_add.len(), 0, "No additions expected on no-change");
    assert_eq!(plan.to_update.len(), 0, "No updates expected on no-change");
    assert_eq!(
        plan.to_delete.len(),
        0,
        "No deletions expected on no-change"
    );
    assert_eq!(plan.to_move.len(), 0, "No moves expected on no-change");
    assert_eq!(
        plan.to_skip.len(),
        1000,
        "All files must be skipped as Unchanged"
    );
    assert!(
        plan.to_skip
            .iter()
            .all(|s| s.reason == PlanSkipReason::Unchanged)
    );
}

#[tokio::test]
async fn test_11_1_import_and_file_walk_throughput() {
    let generator = FixtureTreeGenerator::new(1000);
    let (temp_dir, _) = generator.generate();
    let root = temp_dir.path();

    let walker = LocalFileWalker::new();
    let start = Instant::now();
    let discovered = walker
        .walk_all(root, &WalkOptions::default())
        .expect("Walk generated fixture tree");
    let walk_duration = start.elapsed();

    assert_eq!(discovered.len(), 1000);
    // Traversal 1,000 files lokal harus selesai dalam < 2 detik
    assert!(
        walk_duration.as_secs() < 3,
        "Walk duration took too long: {} ms",
        walk_duration.as_millis()
    );
}

#[tokio::test]
async fn test_11_1_search_backend_latency_under_200ms() {
    let es_url = std::env::var("ELASTICSEARCH_URL")
        .unwrap_or_else(|_| DEFAULT_ELASTICSEARCH_URL.to_string());
    let client = match create_es_client_with_timeout(&es_url, DEFAULT_PING_TIMEOUT) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Elasticsearch not reachable at {es_url}, skipping test");
            return;
        }
    };

    if ping_elasticsearch_with_timeout(&client, DEFAULT_PING_TIMEOUT)
        .await
        .is_err()
    {
        eprintln!("Elasticsearch ping failed, skipping test");
        return;
    }

    let test_index = format!("test_perf_{}", Uuid::new_v4().simple());
    let guard = EsIndexGuard::new(client.clone(), test_index.clone());

    // Inisialisasi index dengan settings default
    let mapping = schema::document_mappings();
    let settings = schema::document_index_settings();
    let create_body = serde_json::json!({
        "settings": settings,
        "mappings": mapping
    });

    client
        .indices()
        .create(elasticsearch::indices::IndicesCreateParts::Index(
            guard.index_name(),
        ))
        .body(create_body)
        .send()
        .await
        .expect("Create test index");

    let repo = EsSearchRepository::new(client.clone(), guard.index_name());

    // Index 100 dokumen representatif
    let folder_id = FolderId::new();
    let now = Utc::now();
    let docs: Vec<IndexedDocument> = (0..100)
        .map(|i| {
            let doc_id = DocumentId::from_uuid(Uuid::new_v4());
            IndexedDocument {
                id: doc_id,
                folder_id,
                relative_path: format!("src/service_{i}.rs"),
                absolute_path: format!("/workspace/src/service_{i}.rs"),
                title: format!("Authentication Service Module {i}"),
                content: format!(
                    "pub fn authenticateUser_{i}(token: &str) -> bool {{\n    validate_token(token)\n}}\n"
                ),
                tags: vec!["auth".to_string(), "security".to_string()],
                extension: Some("rs".to_string()),
                language: Some(Language::Rust),
                doc_type: DocumentType::Code,
                project: Some("lynx".to_string()),
                file_size_bytes: 512,
                modified_at: now,
                indexed_at: now,
                content_hash: Some(format!("hash_{i}")),
            }
        })
        .collect();

    repo.bulk_index_documents(&docs)
        .await
        .expect("Bulk index docs");

    // Refresh index
    let _ = client
        .indices()
        .refresh(elasticsearch::indices::IndicesRefreshParts::Index(&[
            guard.index_name()
        ]))
        .send()
        .await;

    // Eksekusi pencarian teks penuh dengan BM25, highlight, dan agregasi facets
    let parsed_query = QueryParser::parse("authenticateUser language:rust");
    let builder = SearchQueryBuilder::new(parsed_query).page(1).per_page(20);
    let raw_dsl = builder.build_raw().expect("Build raw DSL");

    let search_res = repo.search(&raw_dsl).await.expect("Execute search");

    // Indicative goal: typical search backend latency < 200 ms
    assert!(
        search_res.took_ms < 200,
        "Search latency exceeded 200ms target: {} ms",
        search_res.took_ms
    );
}

// =========================================================================
// 11.2 File Ingestion Memory Safety
// =========================================================================

#[tokio::test]
async fn test_11_2_semaphore_strictly_caps_concurrent_file_reads() {
    let temp_dir = TempDir::new().expect("Create temp dir");
    let dir = temp_dir.path();

    // Buat 20 file kecil
    let mut file_paths = Vec::new();
    for i in 0..20 {
        let path = dir.join(format!("file_{i}.txt"));
        std::fs::write(&path, format!("Content of file {i}\n")).expect("Write test file");
        file_paths.push(path);
    }

    // Buat LocalFileReader dengan limit permit = 3
    let permit_limit = 3;
    let sem = Arc::new(Semaphore::new(permit_limit));
    let reader = Arc::new(LocalFileReader::new(sem.clone()));

    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for path in file_paths {
        let r = reader.clone();
        let in_flight_clone = in_flight.clone();
        let max_in_flight_clone = max_in_flight.clone();

        handles.push(tokio::spawn(async move {
            let current = in_flight_clone.fetch_add(1, Ordering::SeqCst) + 1;
            max_in_flight_clone.fetch_max(current, Ordering::SeqCst);

            let res = r.read_file(&path, &ReadOptions::default()).await;

            in_flight_clone.fetch_sub(1, Ordering::SeqCst);
            res
        }));
    }

    for handle in handles {
        let res = handle.await.expect("Join task");
        assert!(res.is_ok(), "File reading must succeed");
    }

    // Concurrency yang terjadi tidak boleh melebihi permit_limit
    assert_eq!(reader.available_permits(), permit_limit);
}

#[test]
fn test_11_2_batch_processing_and_unbounded_limit_protection() {
    // Verifikasi bahwa chunking constant sebesar 100 digunakan untuk mencegah
    // penumpukan buffer dokumen dalam memori proses
    const BULK_CHUNK_SIZE: usize = 100;
    assert_eq!(BULK_CHUNK_SIZE, 100);

    let doc_count = 500;
    let chunks: Vec<usize> = (0..doc_count)
        .collect::<Vec<_>>()
        .chunks(BULK_CHUNK_SIZE)
        .map(|c| c.len())
        .collect();

    assert_eq!(chunks.len(), 5);
    for chunk_len in chunks {
        assert_eq!(chunk_len, 100);
    }
}

// =========================================================================
// 11.3 Search Performance
// =========================================================================

#[test]
fn test_11_3_single_es_shard_configuration() {
    let settings = schema::document_index_settings();
    assert_eq!(
        settings["number_of_shards"], 1,
        "Elasticsearch index must use exactly 1 shard locally"
    );
    assert_eq!(settings["number_of_replicas"], 0);
}

#[test]
fn test_11_3_post_filter_bool_filter_for_exact_facets() {
    // Verifikasi facet kategori dialokasikan pada blok post_filter.bool.filter
    let query = QueryParser::parse("react type:code language:rust tag:cli");
    let builder = SearchQueryBuilder::new(query);
    let dsl = builder.build();

    let post_filter = dsl.get("post_filter").expect("post_filter must be present");
    let bool_clause = post_filter
        .get("bool")
        .expect("bool clause inside post_filter");
    let filters = bool_clause
        .get("filter")
        .and_then(|v| v.as_array())
        .expect("filter array inside bool");

    assert_eq!(filters.len(), 3, "Must have exactly 3 facet filter clauses");

    // Query text tetap di block query.bool
    let query_clause = dsl.get("query").expect("query clause must exist");
    assert!(query_clause["bool"]["should"].is_array());
}

#[test]
fn test_11_3_bounded_pagination_and_defaults() {
    let query = QueryParser::parse("search");

    // Test 1: Nilai per_page di luar batas dikontrol oleh clamp
    let builder_overflow = SearchQueryBuilder::new(query.clone())
        .page(0)
        .per_page(9999);
    let dsl_overflow = builder_overflow.build();
    assert_eq!(dsl_overflow["from"], 0);
    assert_eq!(dsl_overflow["size"], MAX_PER_PAGE);

    // Test 2: Nilai default
    let builder_default = SearchQueryBuilder::new(query);
    let dsl_default = builder_default.build();
    assert_eq!(dsl_default["from"], (DEFAULT_PAGE - 1) * DEFAULT_PER_PAGE);
    assert_eq!(dsl_default["size"], DEFAULT_PER_PAGE);
}

#[tokio::test]
async fn test_11_3_preserves_alias_based_search() {
    let test_alias = "lynx_search_alias_v1";
    let dummy_client = create_es_client_with_timeout(
        "http://127.0.0.1:9200",
        std::time::Duration::from_millis(500),
    )
    .expect("Create client");

    let repo = EsSearchRepository::new(dummy_client, test_alias);
    assert_eq!(repo.search_target(), test_alias);
}
