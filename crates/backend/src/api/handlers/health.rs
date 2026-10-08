use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use elasticsearch::Elasticsearch;
use sqlx::PgPool;
use std::time::{Duration, Instant};

use crate::api::dtos::{
    ComponentHealthDto, HealthSummaryResponseDto, LivenessResponseDto, ReadinessResponseDto,
    StatsResponseDto,
};
use crate::state::AppState;

pub const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_millis(1500);

pub async fn check_database_health(pool: &PgPool) -> ComponentHealthDto {
    let start = Instant::now();
    let query_fut = sqlx::query("SELECT 1").execute(pool);

    match tokio::time::timeout(HEALTH_CHECK_TIMEOUT, query_fut).await {
        Ok(Ok(_)) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            ComponentHealthDto {
                status: "up".to_string(),
                latency_ms: Some(latency_ms),
                error: None,
            }
        }
        Ok(Err(e)) => ComponentHealthDto {
            status: "down".to_string(),
            latency_ms: None,
            error: Some(e.to_string()),
        },
        Err(_) => ComponentHealthDto {
            status: "down".to_string(),
            latency_ms: None,
            error: Some("Database health check timed out".to_string()),
        },
    }
}

pub async fn check_elasticsearch_health(client: &Elasticsearch) -> ComponentHealthDto {
    let start = Instant::now();
    let ping_fut = client.ping().send();

    match tokio::time::timeout(HEALTH_CHECK_TIMEOUT, ping_fut).await {
        Ok(Ok(response)) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            if response.status_code().is_success() {
                ComponentHealthDto {
                    status: "up".to_string(),
                    latency_ms: Some(latency_ms),
                    error: None,
                }
            } else {
                ComponentHealthDto {
                    status: "down".to_string(),
                    latency_ms: Some(latency_ms),
                    error: Some(format!(
                        "Elasticsearch ping returned status {}",
                        response.status_code()
                    )),
                }
            }
        }
        Ok(Err(e)) => ComponentHealthDto {
            status: "down".to_string(),
            latency_ms: None,
            error: Some(e.to_string()),
        },
        Err(_) => ComponentHealthDto {
            status: "down".to_string(),
            latency_ms: None,
            error: Some("Elasticsearch ping timed out".to_string()),
        },
    }
}

/// GET /api/health
/// Laporan kesehatan sistem lengkap (backend, status DB + latency, status ES + latency).
pub async fn health_summary(State(state): State<AppState>) -> impl IntoResponse {
    let (database, elasticsearch) = tokio::join!(
        check_database_health(&state.db_pool),
        check_elasticsearch_health(&state.es_client)
    );

    let status = if database.status == "up" && elasticsearch.status == "up" {
        "ok"
    } else {
        "degraded"
    };

    let response = HealthSummaryResponseDto {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        database,
        elasticsearch,
    };

    (StatusCode::OK, Json(response))
}

/// GET /api/health/live
/// Liveness probe: hanya memverifikasi proses & router backend aktif.
pub async fn health_live(_state: State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, Json(LivenessResponseDto::default()))
}

/// GET /api/health/ready
/// Readiness probe: memeriksa kesiapan PostgreSQL & Elasticsearch.
/// Return 503 jika salah satu dependency gagal.
pub async fn health_ready(State(state): State<AppState>) -> impl IntoResponse {
    let (database, elasticsearch) = tokio::join!(
        check_database_health(&state.db_pool),
        check_elasticsearch_health(&state.es_client)
    );

    if database.status == "up" && elasticsearch.status == "up" {
        (
            StatusCode::OK,
            Json(ReadinessResponseDto {
                status: "ready".to_string(),
                database: None,
                elasticsearch: None,
            }),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadinessResponseDto {
                status: "unavailable".to_string(),
                database: Some(database),
                elasticsearch: Some(elasticsearch),
            }),
        )
    }
}

/// GET /api/stats
/// Mengambil statistik aktual dari PostgreSQL (folders, document_registry)
/// dan Elasticsearch (agregasi distribusi types & languages).
pub async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    let mut total_documents = 0u64;
    let mut total_size_bytes = 0u64;
    let mut indexed_folders = 0u64;
    let mut types = std::collections::HashMap::new();
    let mut languages = std::collections::HashMap::new();

    // 1. Ambil jumlah folder terdaftar dari database
    if let Ok(folders_count) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM folders")
        .fetch_one(&state.db_pool)
        .await
    {
        indexed_folders = folders_count.max(0) as u64;
    }

    // 2. Ambil dokumen terindeks dan ukuran total dari document_registry
    if let Ok(row) = sqlx::query_as::<_, (i64, i64)>(
        "SELECT 
            COUNT(*) FILTER (WHERE status = 'INDEXED'),
            COALESCE(SUM(file_size_bytes) FILTER (WHERE status = 'INDEXED'), 0)
         FROM document_registry",
    )
    .fetch_one(&state.db_pool)
    .await
    {
        total_documents = row.0.max(0) as u64;
        total_size_bytes = row.1.max(0) as u64;
    }

    // 3. Ambil agregasi tipe & bahasa dari Elasticsearch
    let search_target = state.repositories.search.search_alias();
    let aggs_query = serde_json::json!({
        "size": 0,
        "aggs": {
            "types": {
                "terms": {
                    "field": "type",
                    "size": 100
                }
            },
            "languages": {
                "terms": {
                    "field": "language",
                    "size": 100
                }
            }
        }
    });

    if let Ok(response) = state
        .es_client
        .search(elasticsearch::SearchParts::Index(&[search_target]))
        .body(&aggs_query)
        .send()
        .await
        && response.status_code().is_success()
        && let Ok(json) = response.json::<serde_json::Value>().await
    {
        // Fallback total_documents jika Postgres 0 tapi ES memiliki dokumen
        if total_documents == 0
            && let Some(es_total) = json
                .get("hits")
                .and_then(|h| h.get("total"))
                .and_then(|t| t.get("value"))
                .and_then(|v| v.as_u64())
        {
            total_documents = es_total;
        }

        if let Some(aggs) = json.get("aggregations") {
            if let Some(types_buckets) = aggs
                .get("types")
                .and_then(|t| t.get("buckets"))
                .and_then(|b| b.as_array())
            {
                for bucket in types_buckets {
                    if let (Some(key), Some(count)) = (
                        bucket.get("key").and_then(|k| k.as_str()),
                        bucket.get("doc_count").and_then(|c| c.as_u64()),
                    ) && !key.trim().is_empty()
                    {
                        types.insert(key.to_string(), count);
                    }
                }
            }

            if let Some(lang_buckets) = aggs
                .get("languages")
                .and_then(|l| l.get("buckets"))
                .and_then(|b| b.as_array())
            {
                for bucket in lang_buckets {
                    if let (Some(key), Some(count)) = (
                        bucket.get("key").and_then(|k| k.as_str()),
                        bucket.get("doc_count").and_then(|c| c.as_u64()),
                    ) && !key.trim().is_empty()
                    {
                        languages.insert(key.to_string(), count);
                    }
                }
            }
        }
    }

    let stats_dto = StatsResponseDto {
        total_documents,
        total_size_bytes,
        types,
        languages,
        indexed_folders,
    };

    (StatusCode::OK, Json(stats_dto))
}
