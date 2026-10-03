pub mod api;
pub mod application;
pub mod config;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod state;

pub use api::routes::{create_router, create_router_with_state, create_router_with_timeout};
pub use application::{
    JobProgressState, JobTracker, RecoveryReport, SHUTDOWN_CANCEL_REASON,
    STARTUP_CRASH_RECOVERY_ERROR_MSG, ShutdownReport, WorkerCommand,
    drain_worker_with_grace_period, recover_on_startup, shutdown_in_flight_jobs, shutdown_signal,
};
pub use config::{AppConfig, Bm25Weights, ConfigError};
pub use domain::models::AppSettings;
pub use error::{AppError, ErrorCode, ErrorResponse};
pub use state::{AppState, Repositories};
