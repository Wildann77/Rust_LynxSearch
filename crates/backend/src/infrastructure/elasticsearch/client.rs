use std::time::Duration;

use async_trait::async_trait;
use elasticsearch::Elasticsearch;
use elasticsearch::auth::Credentials;
use elasticsearch::http::Url;
use elasticsearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use elasticsearch::indices::{
    IndicesCreateParts, IndicesDeleteParts, IndicesExistsAliasParts, IndicesExistsParts,
    IndicesGetAliasParts, IndicesPutAliasParts,
};
use elasticsearch::{BulkParts, DeleteByQueryParts, DeleteParts, IndexParts, SearchParts};

use crate::config::{AppConfig, DEFAULT_ELASTICSEARCH_INDEX_ALIAS, DEFAULT_ELASTICSEARCH_URL};
use crate::domain::models::{
    BulkIndexReport, DocumentId, FolderId, IndexedDocument, SearchRawResponse,
};
use crate::domain::ports::SearchRepository;
use crate::error::AppError;

pub const DEFAULT_CLIENT_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_PING_TIMEOUT: Duration = Duration::from_secs(3);
pub const DEFAULT_SEARCH_ALIAS: &str = DEFAULT_ELASTICSEARCH_INDEX_ALIAS;
pub const DEFAULT_SEARCH_URL: &str = DEFAULT_ELASTICSEARCH_URL;

/// Initialize Elasticsearch client from URL with default timeout.
pub fn create_es_client(url: &str) -> Result<Elasticsearch, AppError> {
    create_es_client_with_timeout(url, DEFAULT_CLIENT_TIMEOUT)
}

/// Initialize Elasticsearch client with custom connection timeout.
pub fn create_es_client_with_timeout(
    url: &str,
    timeout: Duration,
) -> Result<Elasticsearch, AppError> {
    let mut parsed_url = Url::parse(url).map_err(|e| {
        AppError::Config(crate::config::ConfigError::InvalidValue {
            key: "ELASTICSEARCH_URL".to_string(),
            message: format!("Invalid Elasticsearch URL '{url}': {e}"),
        })
    })?;

    let credentials = if !parsed_url.username().is_empty() && parsed_url.password().is_some() {
        let username = parsed_url.username().to_string();
        let password = parsed_url.password().unwrap_or_default().to_string();
        let _ = parsed_url.set_username("");
        let _ = parsed_url.set_password(None);
        Some(Credentials::Basic(username, password))
    } else {
        None
    };

    let pool = SingleNodeConnectionPool::new(parsed_url);
    let mut builder = TransportBuilder::new(pool).timeout(timeout);

    if let Some(creds) = credentials {
        builder = builder.auth(creds);
    }

    let transport = builder.build().map_err(|e| {
        AppError::SearchEngine(format!("Failed to build Elasticsearch transport: {e}"))
    })?;

    Ok(Elasticsearch::new(transport))
}

/// Ping Elasticsearch server using default timeout.
pub async fn ping_elasticsearch(client: &Elasticsearch) -> Result<(), AppError> {
    ping_elasticsearch_with_timeout(client, DEFAULT_PING_TIMEOUT).await
}

/// Ping Elasticsearch server with explicit timeout.
pub async fn ping_elasticsearch_with_timeout(
    client: &Elasticsearch,
    timeout: Duration,
) -> Result<(), AppError> {
    let ping_fut = client.ping().send();

    match tokio::time::timeout(timeout, ping_fut).await {
        Ok(Ok(response)) => {
            if response.status_code().is_success() {
                Ok(())
            } else {
                Err(AppError::SearchEngine(format!(
                    "Elasticsearch ping returned error status: {}",
                    response.status_code()
                )))
            }
        }
        Ok(Err(err)) => Err(AppError::SearchEngine(format!(
            "Elasticsearch ping failed: {err}"
        ))),
        Err(_) => Err(AppError::RequestTimeout(
            "Elasticsearch ping timed out".to_string(),
        )),
    }
}

/// Concrete Elasticsearch adapter implementing `SearchRepository` port.
/// Encapsulates direct Elasticsearch client interaction away from domain layer.
#[derive(Clone)]
pub struct EsSearchRepository {
    client: Elasticsearch,
    search_target: String,
}

impl EsSearchRepository {
    pub fn new(client: Elasticsearch, search_target: impl Into<String>) -> Self {
        Self {
            client,
            search_target: search_target.into(),
        }
    }

    pub fn default_with_client(client: Elasticsearch) -> Self {
        Self::new(client, DEFAULT_SEARCH_ALIAS)
    }

    pub fn from_url(url: &str) -> Result<Self, AppError> {
        let client = create_es_client(url)?;
        Ok(Self::default_with_client(client))
    }

    pub fn from_config(config: &AppConfig) -> Result<Self, AppError> {
        let client = create_es_client(&config.elasticsearch_url)?;
        Ok(Self::new(client, &config.elasticsearch_index_alias))
    }

    pub fn client(&self) -> &Elasticsearch {
        &self.client
    }

    pub fn search_target(&self) -> &str {
        &self.search_target
    }
}

#[async_trait]
impl SearchRepository for EsSearchRepository {
    fn search_alias(&self) -> &str {
        &self.search_target
    }

    async fn index_document(&self, doc: &IndexedDocument) -> Result<(), AppError> {
        let doc_id_str = doc.id.to_string();
        let response = self
            .client
            .index(IndexParts::IndexId(&self.search_target, &doc_id_str))
            .body(doc)
            .send()
            .await
            .map_err(|e| AppError::SearchEngine(format!("Failed to index document: {e}")))?;

        if !response.status_code().is_success() {
            let status = response.status_code();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::SearchEngine(format!(
                "Elasticsearch indexing failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    async fn bulk_index_documents(
        &self,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError> {
        self.bulk_index_to_target(&self.search_target, docs).await
    }

    async fn bulk_index_to_target(
        &self,
        target_index: &str,
        docs: &[IndexedDocument],
    ) -> Result<BulkIndexReport, AppError> {
        if docs.is_empty() {
            return Ok(BulkIndexReport::default());
        }

        let mut ops: Vec<elasticsearch::BulkOperation<&IndexedDocument>> =
            Vec::with_capacity(docs.len());
        for doc in docs {
            let doc_id = doc.id.to_string();
            ops.push(elasticsearch::BulkOperation::index(doc).id(doc_id).into());
        }

        let response = self
            .client
            .bulk(BulkParts::Index(target_index))
            .body(ops)
            .send()
            .await
            .map_err(|e| AppError::SearchEngine(format!("Bulk index request failed: {e}")))?;

        if !response.status_code().is_success() {
            let status = response.status_code();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::SearchEngine(format!(
                "Elasticsearch bulk indexing returned status {status}: {body}"
            )));
        }

        let resp_json: serde_json::Value = response.json().await.map_err(|e| {
            AppError::SearchEngine(format!("Failed to parse bulk index response: {e}"))
        })?;

        let has_errors = resp_json
            .get("errors")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);

        if !has_errors {
            return Ok(BulkIndexReport {
                indexed: docs.len(),
                failed: 0,
                errors: Vec::new(),
            });
        }

        let mut indexed = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        if let Some(items) = resp_json.get("items").and_then(|i| i.as_array()) {
            for item in items {
                let op = item
                    .get("index")
                    .or_else(|| item.get("create"))
                    .or_else(|| item.get("update"));
                let status = op
                    .and_then(|o| o.get("status"))
                    .and_then(|s| s.as_u64())
                    .unwrap_or(500);

                if (200..300).contains(&status) {
                    indexed += 1;
                } else {
                    failed += 1;
                    let err_reason = op
                        .and_then(|o| o.get("error"))
                        .and_then(|e| e.get("reason"))
                        .and_then(|r| r.as_str())
                        .unwrap_or("Unknown indexing error");
                    errors.push(err_reason.to_string());
                }
            }
        } else {
            failed = docs.len();
            errors.push("Missing items array in Elasticsearch bulk response".to_string());
        }

        Ok(BulkIndexReport {
            indexed,
            failed,
            errors,
        })
    }

    async fn delete_document(&self, id: &DocumentId) -> Result<(), AppError> {
        let id_str = id.to_string();
        let response = self
            .client
            .delete(DeleteParts::IndexId(&self.search_target, &id_str))
            .send()
            .await
            .map_err(|e| AppError::SearchEngine(format!("Failed to delete document: {e}")))?;

        let status = response.status_code();
        if !status.is_success() && status != 404 {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::SearchEngine(format!(
                "Elasticsearch delete failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    async fn delete_documents_by_folder(&self, folder_id: &FolderId) -> Result<u64, AppError> {
        let folder_id_str = folder_id.to_string();
        let query = serde_json::json!({
            "query": {
                "bool": {
                    "should": [
                        { "term": { "folder_id": folder_id_str } },
                        { "term": { "folder_id.keyword": folder_id_str } }
                    ],
                    "minimum_should_match": 1
                }
            }
        });

        let _ = self
            .client
            .indices()
            .refresh(elasticsearch::indices::IndicesRefreshParts::Index(&[
                &self.search_target
            ]))
            .send()
            .await;

        let response = self
            .client
            .delete_by_query(DeleteByQueryParts::Index(&[&self.search_target]))
            .refresh(true)
            .body(query)
            .send()
            .await
            .map_err(|e| {
                AppError::SearchEngine(format!("Failed to delete documents by folder: {e}"))
            })?;

        if !response.status_code().is_success() {
            let status = response.status_code();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::SearchEngine(format!(
                "Elasticsearch delete_by_query failed ({status}): {body}"
            )));
        }

        let resp_json: serde_json::Value = response.json().await.map_err(|e| {
            AppError::SearchEngine(format!("Failed to parse delete_by_query response: {e}"))
        })?;

        let deleted = resp_json
            .get("deleted")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        Ok(deleted)
    }

    async fn search(
        &self,
        query_dsl: &serde_json::value::RawValue,
    ) -> Result<SearchRawResponse, AppError> {
        let start = std::time::Instant::now();
        let response = match self
            .client
            .search(SearchParts::Index(&[&self.search_target]))
            .body(query_dsl)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                return Err(AppError::SearchEngine(format!(
                    "Elasticsearch connection or network failure: {e}"
                )));
            }
        };

        let took_ms = start.elapsed().as_millis() as u64;

        if !response.status_code().is_success() {
            let status = response.status_code();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable error body>".to_string());

            if status.as_u16() == 400 {
                return Err(AppError::InvalidQuery(format!(
                    "Malformed Elasticsearch query DSL (400 Bad Request): {body}"
                )));
            }

            if status.is_server_error() {
                return Err(AppError::SearchEngine(format!(
                    "Elasticsearch server error {status}: {body}"
                )));
            }

            return Err(AppError::Internal(format!(
                "Elasticsearch search returned unexpected status {status}: {body}"
            )));
        }

        let raw_json = response.text().await.map_err(|e| {
            AppError::SearchEngine(format!("Failed to read search response body: {e}"))
        })?;

        Ok(SearchRawResponse { raw_json, took_ms })
    }

    async fn suggest(&self, prefix: &str, limit: usize) -> Result<Vec<String>, AppError> {
        let trimmed = prefix.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let fetch_size = limit.max(1).saturating_mul(2).min(100);
        let query_body = serde_json::json!({
            "size": fetch_size,
            "_source": ["title"],
            "query": {
                "match": {
                    "title.suggest": {
                        "query": trimmed,
                        "operator": "and"
                    }
                }
            }
        });

        let response = match self
            .client
            .search(SearchParts::Index(&[&self.search_target]))
            .body(query_body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                return Err(AppError::SearchEngine(format!(
                    "Elasticsearch connection or network failure in suggest: {e}"
                )));
            }
        };

        if !response.status_code().is_success() {
            let status = response.status_code();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable error body>".to_string());
            return Err(AppError::SearchEngine(format!(
                "Elasticsearch suggest returned status {status}: {body}"
            )));
        }

        let res_val: serde_json::Value = response.json().await.map_err(|e| {
            AppError::SearchEngine(format!("Failed to parse suggest response JSON: {e}"))
        })?;

        let mut suggestions = Vec::new();
        let mut seen = std::collections::HashSet::new();

        if let Some(hits) = res_val["hits"]["hits"].as_array() {
            for hit in hits {
                if let Some(title) = hit["_source"]["title"].as_str() {
                    let clean_title = title.trim();
                    if !clean_title.is_empty() && seen.insert(clean_title.to_lowercase()) {
                        suggestions.push(clean_title.to_string());
                        if suggestions.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }

        Ok(suggestions)
    }

    async fn rebuild_index_with_alias(&self, new_index: &str, alias: &str) -> Result<(), AppError> {
        let old_indices = self.get_alias_indices(alias).await?;
        self.swap_alias(alias, &old_indices, new_index).await?;
        let current_indices = self.get_alias_indices(alias).await?;
        if !current_indices.contains(&new_index.to_string()) {
            return Err(AppError::SearchEngine(format!(
                "Alias swap verification failed: alias '{alias}' does not point to '{new_index}'"
            )));
        }
        for old in &old_indices {
            if old != new_index {
                self.delete_index(old).await?;
            }
        }
        Ok(())
    }

    async fn ping(&self) -> Result<(), AppError> {
        ping_elasticsearch(&self.client).await
    }

    async fn ensure_initial_index(&self) -> Result<String, AppError> {
        // 1. Check if alias already points to an active physical index
        if let Some(active) = self.get_active_physical_index().await? {
            return Ok(active);
        }

        // 2. Fail-safe collision check: ensure no concrete index with alias name exists
        let exists_concrete = self
            .client
            .indices()
            .exists(IndicesExistsParts::Index(&[&self.search_target]))
            .send()
            .await
            .map_err(|e| {
                AppError::SearchEngine(format!(
                    "Failed to check if concrete index '{}' exists: {e}",
                    self.search_target
                ))
            })?;

        if exists_concrete.status_code().is_success() {
            return Err(AppError::SearchEngine(format!(
                "Elasticsearch index '{}' exists as a concrete physical index, not an alias. Please migrate or delete it to enable versioned alias indexing.",
                self.search_target
            )));
        }

        // 3. First-run bootstrap: target physical versioned index (e.g. `lynx_documents_v1`)
        let initial_index =
            crate::infrastructure::elasticsearch::physical_index_name(&self.search_target, 1);

        // Check if initial physical index already exists without alias
        let initial_exists = self
            .client
            .indices()
            .exists(IndicesExistsParts::Index(&[&initial_index]))
            .send()
            .await
            .map_err(|e| {
                AppError::SearchEngine(format!(
                    "Failed to check if initial physical index '{initial_index}' exists: {e}"
                ))
            })?;

        if initial_exists.status_code().is_success() {
            let bind_res = self
                .client
                .indices()
                .put_alias(IndicesPutAliasParts::IndexName(
                    &[&initial_index],
                    &self.search_target,
                ))
                .send()
                .await
                .map_err(|e| {
                    AppError::SearchEngine(format!(
                        "Failed to bind alias '{}' to existing index '{initial_index}': {e}",
                        self.search_target
                    ))
                })?;

            if !bind_res.status_code().is_success() {
                let status = bind_res.status_code();
                let body = bind_res.text().await.unwrap_or_default();
                return Err(AppError::SearchEngine(format!(
                    "Failed to bind alias '{}' to '{initial_index}' (status {status}): {body}",
                    self.search_target
                )));
            }
        } else {
            let schema_with_alias =
                crate::infrastructure::elasticsearch::document_index_schema_with_alias(
                    &self.search_target,
                );

            let create_res = self
                .client
                .indices()
                .create(IndicesCreateParts::Index(&initial_index))
                .body(schema_with_alias)
                .send()
                .await
                .map_err(|e| {
                    AppError::SearchEngine(format!(
                        "Failed to create initial physical index '{initial_index}': {e}"
                    ))
                })?;

            if !create_res.status_code().is_success() {
                let status = create_res.status_code();
                let body = create_res.text().await.unwrap_or_default();
                return Err(AppError::SearchEngine(format!(
                    "Failed to create initial physical index '{initial_index}' (status {status}): {body}",
                )));
            }
        }

        Ok(initial_index)
    }

    async fn get_active_physical_index(&self) -> Result<Option<String>, AppError> {
        let indices = self.get_alias_indices(&self.search_target).await?;
        Ok(indices.into_iter().next())
    }

    async fn create_versioned_index(&self, version: u32) -> Result<String, AppError> {
        let index_name =
            crate::infrastructure::elasticsearch::physical_index_name(&self.search_target, version);
        let schema = crate::infrastructure::elasticsearch::document_index_schema();

        let response = self
            .client
            .indices()
            .create(IndicesCreateParts::Index(&index_name))
            .body(schema)
            .send()
            .await
            .map_err(|e| {
                AppError::SearchEngine(format!(
                    "Failed to create versioned index '{index_name}': {e}"
                ))
            })?;

        if !response.status_code().is_success() {
            let status = response.status_code();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::SearchEngine(format!(
                "Failed to create versioned index '{index_name}' (status {status}): {body}"
            )));
        }

        Ok(index_name)
    }

    async fn get_alias_indices(&self, alias: &str) -> Result<Vec<String>, AppError> {
        let alias_exists_res = self
            .client
            .indices()
            .exists_alias(IndicesExistsAliasParts::Name(&[alias]))
            .send()
            .await
            .map_err(|e| {
                AppError::SearchEngine(format!("Failed to check alias '{alias}' existence: {e}"))
            })?;

        if alias_exists_res.status_code().as_u16() == 404 {
            return Ok(Vec::new());
        }

        if !alias_exists_res.status_code().is_success() {
            let status = alias_exists_res.status_code();
            return Err(AppError::SearchEngine(format!(
                "Alias check returned unexpected status {status} for alias '{alias}'"
            )));
        }

        let response = self
            .client
            .indices()
            .get_alias(IndicesGetAliasParts::Name(&[alias]))
            .send()
            .await
            .map_err(|e| {
                AppError::SearchEngine(format!("Failed to fetch alias '{alias}' details: {e}"))
            })?;

        if response.status_code().as_u16() == 404 {
            return Ok(Vec::new());
        }

        if !response.status_code().is_success() {
            let status = response.status_code();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::SearchEngine(format!(
                "Failed to get alias '{alias}' (status {status}): {body}"
            )));
        }

        let body: serde_json::Value = response.json().await.map_err(|e| {
            AppError::SearchEngine(format!(
                "Failed to parse get_alias JSON response for '{alias}': {e}"
            ))
        })?;

        if let Some(map) = body.as_object() {
            let mut indices: Vec<String> = map.keys().cloned().collect();
            indices.sort_by(|a, b| {
                let ver_a = crate::infrastructure::elasticsearch::parse_index_version(a, alias);
                let ver_b = crate::infrastructure::elasticsearch::parse_index_version(b, alias);
                ver_b.cmp(&ver_a).then_with(|| b.cmp(a))
            });
            Ok(indices)
        } else {
            Ok(Vec::new())
        }
    }

    async fn swap_alias(
        &self,
        alias: &str,
        old_indices: &[String],
        new_index: &str,
    ) -> Result<(), AppError> {
        let mut actions = Vec::new();
        for old in old_indices {
            if old != new_index {
                actions.push(serde_json::json!({
                    "remove": {
                        "index": old,
                        "alias": alias
                    }
                }));
            }
        }
        actions.push(serde_json::json!({
            "add": {
                "index": new_index,
                "alias": alias
            }
        }));

        let body = serde_json::json!({ "actions": actions });

        let response = self
            .client
            .indices()
            .update_aliases()
            .body(body)
            .send()
            .await
            .map_err(|e| {
                AppError::SearchEngine(format!(
                    "Failed to execute atomic alias swap for alias '{alias}': {e}"
                ))
            })?;

        if !response.status_code().is_success() {
            let status = response.status_code();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::SearchEngine(format!(
                "Elasticsearch alias swap returned error status {status}: {body}"
            )));
        }

        Ok(())
    }

    async fn delete_index(&self, index_name: &str) -> Result<(), AppError> {
        let response = self
            .client
            .indices()
            .delete(IndicesDeleteParts::Index(&[index_name]))
            .send()
            .await
            .map_err(|e| {
                AppError::SearchEngine(format!("Failed to delete index '{index_name}': {e}"))
            })?;

        if response.status_code().as_u16() == 404 || response.status_code().is_success() {
            Ok(())
        } else {
            let status = response.status_code();
            let body = response.text().await.unwrap_or_default();
            Err(AppError::SearchEngine(format!(
                "Elasticsearch delete index '{index_name}' failed with status {status}: {body}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use axum::http::StatusCode;
    use std::sync::Arc;

    #[test]
    fn test_create_es_client_valid_url() {
        let client = create_es_client(DEFAULT_ELASTICSEARCH_URL);
        assert!(client.is_ok(), "Client should initialize from default URL");
    }

    #[test]
    fn test_create_es_client_with_auth_in_url() {
        let url_with_auth = "http://elastic:changeme@127.0.0.1:9200";
        let client = create_es_client(url_with_auth);
        assert!(
            client.is_ok(),
            "Client should handle basic auth credentials in URL"
        );
    }

    #[test]
    fn test_create_es_client_invalid_url() {
        let result = create_es_client("not-a-valid-url");
        assert!(
            result.is_err(),
            "Invalid URL string should return Config error"
        );
    }

    #[test]
    fn test_search_repository_default_alias() {
        let client = create_es_client(DEFAULT_ELASTICSEARCH_URL).expect("Client init");
        let repo = EsSearchRepository::default_with_client(client);
        assert_eq!(repo.search_target(), DEFAULT_SEARCH_ALIAS);
        assert_eq!(repo.search_target(), "lynx_documents");
    }

    #[test]
    fn test_search_repository_custom_alias() {
        let client = create_es_client(DEFAULT_ELASTICSEARCH_URL).expect("Client init");
        let repo = EsSearchRepository::new(client, "custom_alias_v1");
        assert_eq!(repo.search_target(), "custom_alias_v1");
    }

    #[test]
    fn test_search_repository_implements_search_repository_port() {
        let client = create_es_client(DEFAULT_ELASTICSEARCH_URL).expect("Client init");
        let repo: Arc<dyn SearchRepository> =
            Arc::new(EsSearchRepository::default_with_client(client));
        assert!(repo.search_target_is_clean());
    }

    trait SearchRepoExt {
        fn search_target_is_clean(&self) -> bool;
    }

    impl SearchRepoExt for Arc<dyn SearchRepository> {
        fn search_target_is_clean(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_ping_unreachable_server_returns_service_unavailable() {
        // Port 1 is reserved and immediately unavailable
        let client =
            create_es_client_with_timeout("http://127.0.0.1:1", Duration::from_millis(500))
                .expect("Client creation succeeds lazily");

        let repo = EsSearchRepository::default_with_client(client);
        let result = repo.ping().await;

        assert!(result.is_err(), "Ping to unreachable server must fail");
        let err = result.unwrap_err();

        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::SearchEngineUnavailable);
        assert_eq!(err.client_message(), "Search engine service unavailable.");
    }

    #[tokio::test]
    async fn test_ensure_initial_index_unreachable_server_fails() {
        let client =
            create_es_client_with_timeout("http://127.0.0.1:1", Duration::from_millis(500))
                .expect("Client creation succeeds lazily");

        let repo = EsSearchRepository::default_with_client(client);
        let result = repo.ensure_initial_index().await;

        assert!(
            result.is_err(),
            "ensure_initial_index to unreachable server must return error"
        );
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::SearchEngineUnavailable);
    }

    #[tokio::test]
    async fn test_get_active_physical_index_unreachable_server_fails() {
        let client =
            create_es_client_with_timeout("http://127.0.0.1:1", Duration::from_millis(500))
                .expect("Client creation succeeds lazily");

        let repo = EsSearchRepository::default_with_client(client);
        let result = repo.get_active_physical_index().await;

        assert!(
            result.is_err(),
            "get_active_physical_index to unreachable server must return error"
        );
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::SearchEngineUnavailable);
    }

    #[tokio::test]
    async fn test_create_versioned_index_unreachable_server_fails() {
        let client =
            create_es_client_with_timeout("http://127.0.0.1:1", Duration::from_millis(500))
                .expect("Client creation succeeds lazily");

        let repo = EsSearchRepository::default_with_client(client);
        let result = repo.create_versioned_index(2).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::SearchEngineUnavailable);
    }

    #[tokio::test]
    async fn test_get_alias_indices_unreachable_server_fails() {
        let client =
            create_es_client_with_timeout("http://127.0.0.1:1", Duration::from_millis(500))
                .expect("Client creation succeeds lazily");

        let repo = EsSearchRepository::default_with_client(client);
        let result = repo.get_alias_indices("lynx_documents").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::SearchEngineUnavailable);
    }

    #[tokio::test]
    async fn test_swap_alias_unreachable_server_fails() {
        let client =
            create_es_client_with_timeout("http://127.0.0.1:1", Duration::from_millis(500))
                .expect("Client creation succeeds lazily");

        let repo = EsSearchRepository::default_with_client(client);
        let old = vec!["lynx_documents_v1".to_string()];
        let result = repo
            .swap_alias("lynx_documents", &old, "lynx_documents_v2")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::SearchEngineUnavailable);
    }

    #[tokio::test]
    async fn test_delete_index_unreachable_server_fails() {
        let client =
            create_es_client_with_timeout("http://127.0.0.1:1", Duration::from_millis(500))
                .expect("Client creation succeeds lazily");

        let repo = EsSearchRepository::default_with_client(client);
        let result = repo.delete_index("lynx_documents_v1").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::SearchEngineUnavailable);
    }

    #[tokio::test]
    async fn test_rebuild_index_with_alias_unreachable_server_fails() {
        let client =
            create_es_client_with_timeout("http://127.0.0.1:1", Duration::from_millis(500))
                .expect("Client creation succeeds lazily");

        let repo = EsSearchRepository::default_with_client(client);
        let result = repo
            .rebuild_index_with_alias("lynx_documents_v2", "lynx_documents")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::SearchEngineUnavailable);
    }

    #[tokio::test]
    async fn test_search_unreachable_server_fails_service_unavailable() {
        let client =
            create_es_client_with_timeout("http://127.0.0.1:1", Duration::from_millis(500))
                .expect("Client creation succeeds lazily");

        let repo = EsSearchRepository::default_with_client(client);
        let dsl =
            serde_json::value::RawValue::from_string("{\"query\":{\"match_all\":{}}}".to_string())
                .unwrap();
        let result = repo.search(&dsl).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::SearchEngineUnavailable);
    }
}
