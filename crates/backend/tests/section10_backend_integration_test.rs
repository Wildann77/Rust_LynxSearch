use backend::config::DEFAULT_ELASTICSEARCH_URL;
use backend::domain::models::{DocumentId, DocumentType, FolderId, IndexedDocument, Language};
use backend::domain::ports::SearchRepository;
use backend::domain::query_parser::QueryParser;
use backend::domain::services::search_query_builder::SearchQueryBuilder;
use backend::infrastructure::elasticsearch::{
    DEFAULT_PING_TIMEOUT, EsSearchRepository, create_es_client_with_timeout,
    ping_elasticsearch_with_timeout,
};
use backend::infrastructure::postgres::{create_pg_pool_eager, run_migrations};
use chrono::Utc;
use elasticsearch::Elasticsearch;
use elasticsearch::indices::{IndicesDeleteParts, IndicesRefreshParts};
use std::time::Duration;
use uuid::Uuid;

/// RAII Guard that deletes the dynamic Elasticsearch index on drop.
pub struct EsIndexGuard {
    client: Elasticsearch,
    index_name: String,
}

impl EsIndexGuard {
    pub fn new(client: Elasticsearch, index_name: String) -> Self {
        Self { client, index_name }
    }

    pub fn index_name(&self) -> &str {
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

async fn get_real_postgres_pool() -> Option<sqlx::PgPool> {
    let _ = dotenvy::dotenv();
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://lynx:lynxpass@127.0.0.1:5432/lynxsearch".to_string());

    let pool = match create_pg_pool_eager(&db_url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("PostgreSQL not reachable at {db_url}: {e}");
            return None;
        }
    };

    run_migrations(&pool).await.expect("Run migrations cleanly");
    Some(pool)
}

async fn get_real_es_client() -> Option<Elasticsearch> {
    let _ = dotenvy::dotenv();
    let es_url = std::env::var("ELASTICSEARCH_URL")
        .unwrap_or_else(|_| DEFAULT_ELASTICSEARCH_URL.to_string());

    let client = match create_es_client_with_timeout(&es_url, Duration::from_secs(10)) {
        Ok(c) => c,
        Err(_) => return None,
    };

    if ping_elasticsearch_with_timeout(&client, DEFAULT_PING_TIMEOUT)
        .await
        .is_err()
    {
        return None;
    }

    Some(client)
}

#[tokio::test]
async fn test_10_3_isolated_postgres_db_state() {
    let pool = match get_real_postgres_pool().await {
        Some(p) => p,
        None => return,
    };

    // Use transaction for isolated test state
    let mut tx = pool
        .begin()
        .await
        .expect("Begin transaction for isolated test");

    let folder_id = FolderId::new();
    let unique_path = format!("/isolated/test/path/{}", Uuid::new_v4());

    sqlx::query!(
        "INSERT INTO folders (id, root_path, status) VALUES ($1, $2, 'IDLE')",
        folder_id.as_uuid(),
        unique_path
    )
    .execute(&mut *tx)
    .await
    .expect("Insert in transaction");

    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM folders WHERE id = $1",
        folder_id.as_uuid()
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Fetch count inside tx");

    assert_eq!(count, Some(1));

    // Rollback to guarantee no leftover data
    tx.rollback().await.expect("Rollback isolated tx");

    let count_after = sqlx::query_scalar!(
        "SELECT count(*) FROM folders WHERE id = $1",
        folder_id.as_uuid()
    )
    .fetch_one(&pool)
    .await
    .expect("Fetch count outside tx");

    assert_eq!(
        count_after,
        Some(0),
        "No leftover data must remain in database"
    );
}

#[tokio::test]
async fn test_10_3_real_elasticsearch_dynamic_index_with_raii_guard() {
    let client = match get_real_es_client().await {
        Some(c) => c,
        None => return,
    };

    let test_uuid = Uuid::new_v4();
    let dynamic_index = format!("test_lynx_{}", test_uuid.simple());

    // RAII guard will delete index on drop
    let guard = EsIndexGuard::new(client.clone(), dynamic_index.clone());

    let repo = EsSearchRepository::new(client.clone(), guard.index_name());

    let initial = repo
        .ensure_initial_index()
        .await
        .expect("Create index with mapping");
    assert!(initial.starts_with(&dynamic_index));

    let folder_id = FolderId::new();
    let doc_id = DocumentId::from_relative_path(folder_id, "main.rs");
    let now = Utc::now();
    let doc = IndexedDocument {
        id: doc_id,
        folder_id,
        relative_path: "main.rs".to_string(),
        absolute_path: "/work/main.rs".to_string(),
        extension: Some("rs".to_string()),
        doc_type: DocumentType::Code,
        language: Some(Language::Rust),
        project: None,
        tags: vec!["backend".to_string()],
        title: "Main Entrypoint".to_string(),
        content: "fn main() { println!(\"Hello Lynx\"); }".to_string(),
        file_size_bytes: 42,
        modified_at: now,
        indexed_at: now,
        content_hash: Some("sha256_mock".to_string()),
    };

    repo.index_document(&doc)
        .await
        .expect("Index document in dynamic index");

    // Explicit refresh to make document searchable immediately
    client
        .indices()
        .refresh(IndicesRefreshParts::Index(&[&initial]))
        .send()
        .await
        .expect("Refresh dynamic index");

    // Search document using SearchQueryBuilder
    let query = QueryParser::parse("Entrypoint");
    let raw_dsl = SearchQueryBuilder::new(query)
        .build_raw()
        .expect("Build search raw value");

    let search_res = repo.search(&raw_dsl).await.expect("Search document");
    let json_val: serde_json::Value =
        serde_json::from_str(&search_res.raw_json).expect("Parse ES response json");
    let total = json_val["hits"]["total"]["value"].as_u64().unwrap_or(0);
    assert_eq!(total, 1);

    // Clean up document and guard drops
    repo.delete_document(&doc_id)
        .await
        .expect("Delete document");
    drop(guard);
}
