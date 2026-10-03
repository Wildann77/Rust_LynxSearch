use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use elasticsearch::Elasticsearch;
use sqlx::PgPool;
use std::time::{Duration, Instant};

use crate::api::dtos::{
    ComponentHealthDto, HealthSummaryResponseDto, LivenessResponseDto, ReadinessResponseDto,
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
/// Placeholder metrik sistem.
pub async fn stats(_state: State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "total_documents": 0,
        "total_size_bytes": 0,
        "types": {},
        "languages": {},
        "indexed_folders": 0
    }))
}
