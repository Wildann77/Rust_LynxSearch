use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::AppConfig;

/// Resolves OS-specific persistent log directory.
/// - Linux: ~/.config/lynxsearch/logs
/// - Windows: %APPDATA%\lynxsearch\logs
/// - macOS: ~/Library/Application Support/lynxsearch/logs
pub fn get_log_directory() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lynxsearch")
        .join("logs")
}

/// Cleans up log files in `log_dir` older than `max_age`.
/// Returns count of removed stale log files.
pub fn cleanup_old_logs(log_dir: &Path, max_age: Duration) -> Result<usize, std::io::Error> {
    if !log_dir.exists() {
        return Ok(0);
    }

    let mut deleted_count = 0;
    let now = SystemTime::now();

    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|f| f.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Target daily rolled log files
        if !file_name.starts_with("lynxsearch.log") && !file_name.ends_with(".log") {
            continue;
        }

        if let Ok(metadata) = entry.metadata()
            && let Ok(modified) = metadata.modified()
            && let Ok(age) = now.duration_since(modified)
            && age > max_age
        {
            if let Err(err) = fs::remove_file(&path) {
                tracing::warn!(path = %path.display(), error = %err, "Failed to remove stale log file");
            } else {
                deleted_count += 1;
            }
        }
    }

    Ok(deleted_count)
}

/// Masks sensitive credentials (passwords, tokens) in database and API URLs before logging.
pub fn sanitize_secret_url(url: &str) -> String {
    if let Some(at_idx) = url.find('@')
        && let Some(proto_end) = url.find("://")
    {
        let cred_start = proto_end + 3;
        if cred_start < at_idx {
            let user_pass = &url[cred_start..at_idx];
            if let Some(colon_idx) = user_pass.find(':') {
                let user = &user_pass[..colon_idx];
                return format!(
                    "{}://{}:***@{}",
                    &url[..proto_end],
                    user,
                    &url[at_idx + 1..]
                );
            }
        }
    }
    url.to_string()
}

/// Initializes global tracing subscriber with dual output:
/// 1. Human-readable pretty stdout layer
/// 2. Structured JSON non-blocking daily rolling file layer
///
/// Also performs automatic prune of log files older than 7 days.
pub fn init_telemetry(
    config: &AppConfig,
) -> Result<WorkerGuard, Box<dyn std::error::Error + Send + Sync>> {
    let log_dir = get_log_directory();
    fs::create_dir_all(&log_dir)?;

    // Prune log files older than 7 days on startup
    let max_log_age = Duration::from_secs(7 * 24 * 3600);
    match cleanup_old_logs(&log_dir, max_log_age) {
        Ok(pruned) if pruned > 0 => {
            tracing::info!(
                pruned = pruned,
                "Cleaned up old log files older than 7 days"
            );
        }
        Err(err) => {
            tracing::warn!(error = %err, "Failed to run startup log cleanup");
        }
        _ => {}
    }

    let file_appender = tracing_appender::rolling::daily(&log_dir, "lynxsearch.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.rust_log));

    let is_release = !cfg!(debug_assertions);

    let stdout_pretty = if !is_release {
        Some(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_target(true),
        )
    } else {
        None
    };

    let stdout_json = if is_release {
        Some(
            tracing_subscriber::fmt::layer()
                .json()
                .with_ansi(false)
                .with_target(true),
        )
    } else {
        None
    };

    let json_file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(non_blocking_file);

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_pretty)
        .with(stdout_json)
        .with(json_file_layer)
        .try_init();

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sanitize_secret_url() {
        let raw = "postgres://lynx:supersecretpass@127.0.0.1:5432/lynxsearch";
        let sanitized = sanitize_secret_url(raw);
        assert_eq!(sanitized, "postgres://lynx:***@127.0.0.1:5432/lynxsearch");

        let without_secret = "http://127.0.0.1:9200";
        assert_eq!(sanitize_secret_url(without_secret), without_secret);
    }

    #[test]
    fn test_cleanup_old_logs() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("lynxsearch.log.2025-01-01");
        fs::write(&log_path, "old log line").unwrap();

        // Immediate cleanup with 0 duration deletes file
        let deleted = cleanup_old_logs(dir.path(), Duration::from_secs(0)).unwrap();
        assert_eq!(deleted, 1);
        assert!(!log_path.exists());
    }

    #[test]
    fn test_get_log_directory_has_proper_suffix() {
        let path = get_log_directory();
        assert!(path.ends_with(Path::new("lynxsearch").join("logs")));
    }

    #[test]
    fn test_init_telemetry_succeeds() {
        let config = AppConfig {
            database_url: "postgres://user:pass@127.0.0.1:5432/lynx".to_string(),
            elasticsearch_url: "http://127.0.0.1:9200".to_string(),
            elasticsearch_index_alias: "lynx_documents".to_string(),
            backend_bind_addr: "127.0.0.1:3001".to_string(),
            backend_host: "127.0.0.1".to_string(),
            backend_port: 3001,
            rust_log: "info".to_string(),
            default_max_file_size_bytes: 2048,
            default_ignore_patterns: vec![],
            default_bm25_weights: Default::default(),
            file_read_concurrency_limit: 10,
        };
        let res = init_telemetry(&config);
        assert!(res.is_ok());
    }
}
