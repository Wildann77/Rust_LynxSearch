pub mod api;
pub mod application;
pub mod config;
pub mod domain;
pub mod error;
pub mod infrastructure;

use config::AppConfig;
use error::AppError;

pub async fn run() -> Result<(), AppError> {
    let config = AppConfig::from_env().map_err(AppError::Internal)?;

    let app = api::routes::create_router();

    tracing::info!(
        "LynxSearch Backend listening on {}",
        config.backend_bind_addr
    );
    let listener = tokio::net::TcpListener::bind(&config.backend_bind_addr).await?;
    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(())
}
