use backend::AppState;
use backend::config::AppConfig;
use std::error::Error;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = dotenvy::dotenv();
    let config = AppConfig::from_env().map_err(|e| format!("Config error: {e}"))?;

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.rust_log));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(
        "Starting LynxSearch backend on {}",
        config.backend_bind_addr
    );

    let (app_state, worker_rx) =
        AppState::from_config(config.clone()).map_err(|e| format!("AppState error: {e}"))?;

    // Verify database readiness and apply pending migrations
    match backend::infrastructure::postgres::connection::check_database_readiness(
        &app_state.db_pool,
    )
    .await
    {
        Ok(()) => {
            tracing::info!("PostgreSQL connection verified ready");
            if let Err(e) =
                backend::infrastructure::postgres::connection::run_migrations(&app_state.db_pool)
                    .await
            {
                tracing::error!(error = %e, "Database migration failed on startup");
                return Err(format!("Database migration error: {e}").into());
            }
            tracing::info!("Database migrations applied successfully");
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                code = e.error_code().as_str(),
                "PostgreSQL not ready on startup; server starting in degraded mode"
            );
        }
    }

    // Verify Elasticsearch readiness and ensure initial index + alias
    match app_state.repositories.search.ping().await {
        Ok(()) => {
            tracing::info!("Elasticsearch connection verified ready");
            match app_state.repositories.search.ensure_initial_index().await {
                Ok(active_idx) => {
                    tracing::info!(
                        active_index = %active_idx,
                        "Elasticsearch initial index + alias verified ready"
                    );
                }
                Err(e) => {
                    tracing::error!(error = %e, "Elasticsearch initial index bootstrap failed");
                    return Err(format!("Elasticsearch initial index bootstrap error: {e}").into());
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                code = e.error_code().as_str(),
                "Elasticsearch not ready on startup; server starting in degraded mode"
            );
        }
    }

    let recovery_report = app_state
        .recover_on_startup()
        .await
        .map_err(|e| format!("Startup crash recovery error: {e}"))?;

    if recovery_report.jobs_recovered > 0 || recovery_report.folders_reset > 0 {
        tracing::warn!(
            jobs_recovered = recovery_report.jobs_recovered,
            folders_reset = recovery_report.folders_reset,
            "Startup crash recovery reconciled dangling state"
        );
    } else {
        tracing::info!("Startup crash recovery: no dangling jobs or locked folders found");
    }

    let supervisor_handle = backend::application::orchestrator::spawn_worker_supervisor(
        app_state.repositories.clone(),
        app_state.job_tracker.clone(),
        worker_rx,
        app_state.shutdown_token(),
        backend::application::orchestrator::DEFAULT_SUPERVISOR_BACKOFF,
    );

    let app = backend::create_router_with_state(app_state.clone());

    let listener = tokio::net::TcpListener::bind(&config.backend_bind_addr).await?;
    tracing::info!("Server listening on {}", config.backend_bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(backend::shutdown_signal())
        .await?;

    tracing::info!("Axum server stopped accepting requests, beginning graceful shutdown...");

    let shutdown_report = app_state
        .graceful_shutdown(Some(supervisor_handle), std::time::Duration::from_secs(10))
        .await
        .map_err(|e| format!("Graceful shutdown error: {e}"))?;

    tracing::info!(
        jobs_cancelled = shutdown_report.jobs_cancelled,
        folders_reset = shutdown_report.folders_reset,
        timed_out = shutdown_report.grace_period_timed_out,
        "Server shut down gracefully"
    );

    Ok(())
}
