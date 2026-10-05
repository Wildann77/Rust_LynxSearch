use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use super::MIGRATOR;
use crate::error::AppError;

pub const DEFAULT_MAX_CONNECTIONS: u32 = 20;
pub const DEFAULT_MIN_CONNECTIONS: u32 = 5;
pub const DEFAULT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(3);
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
pub const DEFAULT_MAX_LIFETIME: Duration = Duration::from_secs(30 * 60);

pub fn build_pg_pool_options() -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(DEFAULT_MAX_CONNECTIONS)
        .min_connections(DEFAULT_MIN_CONNECTIONS)
        .acquire_timeout(DEFAULT_ACQUIRE_TIMEOUT)
        .idle_timeout(DEFAULT_IDLE_TIMEOUT)
        .max_lifetime(DEFAULT_MAX_LIFETIME)
}

pub fn create_pg_pool_lazy(database_url: &str) -> Result<PgPool, AppError> {
    build_pg_pool_options()
        .connect_lazy(database_url)
        .map_err(AppError::Database)
}

pub async fn create_pg_pool_eager(database_url: &str) -> Result<PgPool, AppError> {
    build_pg_pool_options()
        .connect(database_url)
        .await
        .map_err(AppError::Database)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| AppError::Database(sqlx::Error::Migrate(Box::new(e))))
}

pub async fn check_database_readiness(pool: &PgPool) -> Result<(), AppError> {
    let check_query = sqlx::query_scalar::<_, i32>("SELECT $1::int")
        .bind(1i32)
        .fetch_one(pool);

    match tokio::time::timeout(DEFAULT_ACQUIRE_TIMEOUT, check_query).await {
        Ok(Ok(val)) => {
            if val == 1 {
                Ok(())
            } else {
                Err(AppError::Internal(
                    "Database readiness probe returned unexpected value".to_string(),
                ))
            }
        }
        Ok(Err(e)) => Err(AppError::Database(e)),
        Err(_) => Err(AppError::RequestTimeout(
            "Database readiness check timed out".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use axum::http::StatusCode;

    #[test]
    fn test_pg_pool_options_configuration_values() {
        let options = build_pg_pool_options();

        assert_eq!(options.get_max_connections(), 20);
        assert_eq!(options.get_min_connections(), 5);
        assert_eq!(options.get_acquire_timeout(), Duration::from_secs(3));
        assert_eq!(options.get_idle_timeout(), Some(Duration::from_secs(600)));
        assert_eq!(options.get_max_lifetime(), Some(Duration::from_secs(1800)));
    }

    #[tokio::test]
    async fn test_create_pg_pool_lazy_success() {
        let url = "postgres://postgres:postgres@127.0.0.1:5432/lynx_search";
        let pool = create_pg_pool_lazy(url);
        assert!(pool.is_ok());
        pool.unwrap().close().await;
    }

    #[tokio::test]
    async fn test_create_pg_pool_eager_failure_returns_structured_error() {
        // Port 1 is reserved and typically immediately refuses or fails
        let invalid_url = "postgres://postgres:postgres@127.0.0.1:1/invalid_db";
        let result = create_pg_pool_eager(invalid_url).await;

        assert!(result.is_err());
        let err = result.unwrap_err();

        match &err {
            AppError::Database(sqlx_err) => {
                assert!(!sqlx_err.to_string().is_empty());
            }
            other => panic!("Expected AppError::Database, got: {other:?}"),
        }

        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::DatabaseUnavailable);
        assert_eq!(err.client_message(), "Database service unavailable.");
    }

    #[tokio::test]
    async fn test_check_database_readiness_failure_on_closed_pool() {
        let url = "postgres://postgres:postgres@127.0.0.1:5432/lynx_search";
        let pool = create_pg_pool_lazy(url).expect("Lazy pool creation should succeed");
        pool.close().await;

        let result = check_database_readiness(&pool).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.error_code(), ErrorCode::DatabaseUnavailable);
        assert_eq!(err.client_message(), "Database service unavailable.");
    }
}
