use std::env;

pub const DEFAULT_MAX_FILE_SIZE_BYTES: u64 = 2 * 1024 * 1024; // 2 MB (2,097,152 bytes)
pub const DEFAULT_ELASTICSEARCH_URL: &str = "http://127.0.0.1:9200";
pub const DEFAULT_ELASTICSEARCH_INDEX_ALIAS: &str = "lynx_documents";
pub const DEFAULT_BACKEND_HOST: &str = "127.0.0.1";
pub const DEFAULT_BACKEND_PORT: u16 = 3001;
pub const DEFAULT_RUST_LOG: &str = "info,backend=debug";
pub const DEFAULT_FILE_READ_CONCURRENCY_LIMIT: usize = 50;
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[".git", "node_modules", "target", "dist", "build"];
pub const DEFAULT_BM25_TITLE_WEIGHT: f32 = 3.0;
pub const DEFAULT_BM25_TAGS_WEIGHT: f32 = 2.0;
pub const DEFAULT_BM25_CONTENT_WEIGHT: f32 = 1.0;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error(
        "Missing required environment variable '{0}'. Please set it in your environment or .env file."
    )]
    MissingRequired(String),

    #[error("Invalid configuration for '{key}': {message}")]
    InvalidValue { key: String, message: String },

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Bm25Weights {
    pub title: f32,
    pub tags: f32,
    pub content: f32,
}

impl Default for Bm25Weights {
    fn default() -> Self {
        Self {
            title: DEFAULT_BM25_TITLE_WEIGHT,
            tags: DEFAULT_BM25_TAGS_WEIGHT,
            content: DEFAULT_BM25_CONTENT_WEIGHT,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppConfig {
    pub database_url: String,
    pub elasticsearch_url: String,
    pub elasticsearch_index_alias: String,
    pub backend_bind_addr: String,
    pub backend_host: String,
    pub backend_port: u16,
    pub rust_log: String,
    pub default_max_file_size_bytes: u64,
    pub default_ignore_patterns: Vec<String>,
    pub default_bm25_weights: Bm25Weights,
    pub file_read_concurrency_limit: usize,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();
        Self::from_lookup(|k| env::var(k).ok())
    }

    pub fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        // 1. database_url (required, no credentials hardcoded in source)
        let raw_db_url = lookup("DATABASE_URL")
            .ok_or_else(|| ConfigError::MissingRequired("DATABASE_URL".to_string()))?;
        let database_url = resolve_interpolation(&raw_db_url, &lookup);
        if database_url.contains("${") {
            return Err(ConfigError::InvalidValue {
                key: "DATABASE_URL".to_string(),
                message: "contains unresolved variable interpolation".to_string(),
            });
        }
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err(ConfigError::InvalidValue {
                key: "DATABASE_URL".to_string(),
                message: "must start with 'postgres://' or 'postgresql://'".to_string(),
            });
        }

        // 2. elasticsearch_url & index alias
        let elasticsearch_url =
            lookup("ELASTICSEARCH_URL").unwrap_or_else(|| DEFAULT_ELASTICSEARCH_URL.to_string());
        if !elasticsearch_url.starts_with("http://") && !elasticsearch_url.starts_with("https://") {
            return Err(ConfigError::InvalidValue {
                key: "ELASTICSEARCH_URL".to_string(),
                message: "must start with 'http://' or 'https://'".to_string(),
            });
        }
        let elasticsearch_index_alias = lookup("ELASTICSEARCH_INDEX_ALIAS")
            .unwrap_or_else(|| DEFAULT_ELASTICSEARCH_INDEX_ALIAS.to_string());
        if elasticsearch_index_alias.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "ELASTICSEARCH_INDEX_ALIAS".to_string(),
                message: "index alias cannot be empty".to_string(),
            });
        }

        // 3. host, port & bind address
        let backend_host =
            lookup("BACKEND_HOST").unwrap_or_else(|| DEFAULT_BACKEND_HOST.to_string());
        let backend_port = if let Some(raw_port) = lookup("BACKEND_PORT") {
            let port = raw_port
                .parse::<u16>()
                .map_err(|e| ConfigError::InvalidValue {
                    key: "BACKEND_PORT".to_string(),
                    message: format!("must be a valid 16-bit unsigned integer: {e}"),
                })?;
            if port == 0 {
                return Err(ConfigError::InvalidValue {
                    key: "BACKEND_PORT".to_string(),
                    message: "port must be greater than 0".to_string(),
                });
            }
            port
        } else {
            DEFAULT_BACKEND_PORT
        };

        let backend_bind_addr =
            lookup("BACKEND_BIND_ADDR").unwrap_or_else(|| format!("{backend_host}:{backend_port}"));

        if backend_bind_addr.starts_with("0.0.0.0")
            || backend_bind_addr.contains("0.0.0.0")
            || backend_host == "0.0.0.0"
        {
            return Err(ConfigError::SecurityViolation(
                "Binding to 0.0.0.0 is forbidden. Localhost binding only (127.0.0.1).".to_string(),
            ));
        }

        backend_bind_addr
            .parse::<std::net::SocketAddr>()
            .map_err(|e| ConfigError::InvalidValue {
                key: "BACKEND_BIND_ADDR".to_string(),
                message: format!("invalid socket address format: {e}"),
            })?;

        // 4. log level
        let rust_log = lookup("RUST_LOG").unwrap_or_else(|| DEFAULT_RUST_LOG.to_string());
        if rust_log.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "RUST_LOG".to_string(),
                message: "log level filter cannot be empty".to_string(),
            });
        }

        // 5. default max file size
        let default_max_file_size_bytes = if let Some(raw_size) =
            lookup("DEFAULT_MAX_FILE_SIZE_BYTES").or_else(|| lookup("MAX_FILE_SIZE_BYTES"))
        {
            let size = raw_size
                .parse::<u64>()
                .map_err(|e| ConfigError::InvalidValue {
                    key: "DEFAULT_MAX_FILE_SIZE_BYTES".to_string(),
                    message: format!("must be a positive integer: {e}"),
                })?;
            if size == 0 {
                return Err(ConfigError::InvalidValue {
                    key: "DEFAULT_MAX_FILE_SIZE_BYTES".to_string(),
                    message: "max file size must be greater than 0 bytes".to_string(),
                });
            }
            size
        } else {
            DEFAULT_MAX_FILE_SIZE_BYTES
        };

        // 6. default ignore patterns
        let default_ignore_patterns = if let Some(raw_patterns) =
            lookup("DEFAULT_IGNORE_PATTERNS").or_else(|| lookup("IGNORE_PATTERNS"))
        {
            let patterns: Vec<String> = raw_patterns
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if patterns.is_empty() {
                return Err(ConfigError::InvalidValue {
                    key: "DEFAULT_IGNORE_PATTERNS".to_string(),
                    message: "at least one ignore pattern must be specified".to_string(),
                });
            }
            patterns
        } else {
            DEFAULT_IGNORE_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect()
        };

        // 7. default BM25 weights
        let parse_weight =
            |key: &str, alt_key: &str, default_val: f32| -> Result<f32, ConfigError> {
                if let Some(raw_w) = lookup(key).or_else(|| lookup(alt_key)) {
                    let w = raw_w
                        .parse::<f32>()
                        .map_err(|e| ConfigError::InvalidValue {
                            key: key.to_string(),
                            message: format!("must be a valid float: {e}"),
                        })?;
                    if !w.is_finite() || w <= 0.0 {
                        return Err(ConfigError::InvalidValue {
                            key: key.to_string(),
                            message: "BM25 weight must be a positive finite number (> 0.0)"
                                .to_string(),
                        });
                    }
                    Ok(w)
                } else {
                    Ok(default_val)
                }
            };

        let title_weight = parse_weight(
            "DEFAULT_BM25_TITLE_WEIGHT",
            "BM25_TITLE_WEIGHT",
            DEFAULT_BM25_TITLE_WEIGHT,
        )?;
        let tags_weight = parse_weight(
            "DEFAULT_BM25_TAGS_WEIGHT",
            "BM25_TAGS_WEIGHT",
            DEFAULT_BM25_TAGS_WEIGHT,
        )?;
        let content_weight = parse_weight(
            "DEFAULT_BM25_CONTENT_WEIGHT",
            "BM25_CONTENT_WEIGHT",
            DEFAULT_BM25_CONTENT_WEIGHT,
        )?;

        let default_bm25_weights = Bm25Weights {
            title: title_weight,
            tags: tags_weight,
            content: content_weight,
        };

        // 8. file-read concurrency limit
        let file_read_concurrency_limit = if let Some(raw_limit) =
            lookup("FILE_READ_CONCURRENCY_LIMIT").or_else(|| lookup("CONCURRENCY_LIMIT"))
        {
            let limit = raw_limit
                .parse::<usize>()
                .map_err(|e| ConfigError::InvalidValue {
                    key: "FILE_READ_CONCURRENCY_LIMIT".to_string(),
                    message: format!("must be a positive integer: {e}"),
                })?;
            if limit == 0 {
                return Err(ConfigError::InvalidValue {
                    key: "FILE_READ_CONCURRENCY_LIMIT".to_string(),
                    message: "concurrency limit must be greater than 0".to_string(),
                });
            }
            limit
        } else {
            DEFAULT_FILE_READ_CONCURRENCY_LIMIT
        };

        Ok(Self {
            database_url,
            elasticsearch_url,
            elasticsearch_index_alias,
            backend_bind_addr,
            backend_host,
            backend_port,
            rust_log,
            default_max_file_size_bytes,
            default_ignore_patterns,
            default_bm25_weights,
            file_read_concurrency_limit,
        })
    }
}

fn resolve_interpolation<F>(raw: &str, lookup: &F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    if !raw.contains("${") {
        return raw.to_string();
    }
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.char_indices().peekable();
    let mut last_idx = 0;
    while let Some((idx, ch)) = chars.next() {
        if ch == '$' && raw[idx..].starts_with("${") {
            result.push_str(&raw[last_idx..idx]);
            if let Some(end_rel) = raw[idx + 2..].find('}') {
                let var_name = &raw[idx + 2..idx + 2 + end_rel];
                if let Some(val) = lookup(var_name) {
                    result.push_str(&val);
                } else {
                    result.push_str(&raw[idx..=idx + 2 + end_rel]);
                }
                last_idx = idx + 2 + end_rel + 1;
                while let Some(&(next_idx, _)) = chars.peek() {
                    if next_idx < last_idx {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
        }
    }
    if last_idx < raw.len() {
        result.push_str(&raw[last_idx..]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_env_map() -> HashMap<&'static str, String> {
        let mut map = HashMap::new();
        map.insert(
            "DATABASE_URL",
            "postgres://lynx:lynxpass@127.0.0.1:5432/lynxsearch".to_string(),
        );
        map
    }

    #[test]
    fn test_default_config_with_valid_database_url() {
        let map = sample_env_map();
        let config = AppConfig::from_lookup(|k| map.get(k).cloned())
            .expect("Config should parse with valid defaults");

        assert_eq!(
            config.database_url,
            "postgres://lynx:lynxpass@127.0.0.1:5432/lynxsearch"
        );
        assert_eq!(config.elasticsearch_url, "http://127.0.0.1:9200");
        assert_eq!(config.elasticsearch_index_alias, "lynx_documents");
        assert_eq!(config.backend_bind_addr, "127.0.0.1:3001");
        assert_eq!(config.backend_host, "127.0.0.1");
        assert_eq!(config.backend_port, 3001);
        assert_eq!(config.rust_log, "info,backend=debug");
        assert_eq!(config.default_max_file_size_bytes, 2_097_152);
        assert_eq!(
            config.default_ignore_patterns,
            vec![".git", "node_modules", "target", "dist", "build"]
        );
        assert_eq!(
            config.default_bm25_weights,
            Bm25Weights {
                title: 3.0,
                tags: 2.0,
                content: 1.0,
            }
        );
        assert_eq!(config.file_read_concurrency_limit, 50);
    }

    #[test]
    fn test_missing_database_url_errors() {
        let empty_map = HashMap::<&str, String>::new();
        let err = AppConfig::from_lookup(|k| empty_map.get(k).cloned()).unwrap_err();

        assert_eq!(
            err,
            ConfigError::MissingRequired("DATABASE_URL".to_string())
        );
    }

    #[test]
    fn test_invalid_database_url_protocol_errors() {
        let mut map = HashMap::<&str, String>::new();
        map.insert(
            "DATABASE_URL",
            "mysql://lynx:lynxpass@127.0.0.1:3306/lynxsearch".to_string(),
        );
        let err = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap_err();

        assert!(matches!(err, ConfigError::InvalidValue { key, .. } if key == "DATABASE_URL"));
    }

    #[test]
    fn test_interpolation_database_url() {
        let mut map = HashMap::<&str, String>::new();
        map.insert("POSTGRES_USER", "custom_lynx".to_string());
        map.insert("POSTGRES_PASSWORD", "custom_pass".to_string());
        map.insert("POSTGRES_DB", "custom_db".to_string());
        map.insert(
            "DATABASE_URL",
            "postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@127.0.0.1:5432/${POSTGRES_DB}"
                .to_string(),
        );

        let config = AppConfig::from_lookup(|k| map.get(k).cloned())
            .expect("Should resolve interpolation in DATABASE_URL");

        assert_eq!(
            config.database_url,
            "postgres://custom_lynx:custom_pass@127.0.0.1:5432/custom_db"
        );
    }

    #[test]
    fn test_unresolved_interpolation_errors() {
        let mut map = HashMap::<&str, String>::new();
        map.insert(
            "DATABASE_URL",
            "postgres://${POSTGRES_USER}:pass@127.0.0.1:5432/db".to_string(),
        );

        let err = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { key, .. } if key == "DATABASE_URL"));
    }

    #[test]
    fn test_custom_bind_addr() {
        let mut map = sample_env_map();
        map.insert("BACKEND_BIND_ADDR", "127.0.0.1:4000".to_string());
        let config = AppConfig::from_lookup(|k| map.get(k).cloned())
            .expect("Config should parse custom bind addr");

        assert_eq!(config.backend_bind_addr, "127.0.0.1:4000");
    }

    #[test]
    fn test_rejects_zero_bind_address() {
        let mut map = sample_env_map();
        map.insert("BACKEND_BIND_ADDR", "0.0.0.0:3001".to_string());
        let err = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap_err();

        assert!(matches!(err, ConfigError::SecurityViolation(_)));
    }

    #[test]
    fn test_rejects_invalid_bind_addr_format() {
        let mut map = sample_env_map();
        map.insert("BACKEND_BIND_ADDR", "invalid_host_format".to_string());
        let err = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap_err();

        assert!(matches!(err, ConfigError::InvalidValue { key, .. } if key == "BACKEND_BIND_ADDR"));
    }

    #[test]
    fn test_rejects_invalid_elasticsearch_url() {
        let mut map = sample_env_map();
        map.insert("ELASTICSEARCH_URL", "ftp://127.0.0.1:9200".to_string());
        let err = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap_err();

        assert!(matches!(err, ConfigError::InvalidValue { key, .. } if key == "ELASTICSEARCH_URL"));
    }

    #[test]
    fn test_custom_max_file_size() {
        let mut map = sample_env_map();
        map.insert(
            "DEFAULT_MAX_FILE_SIZE_BYTES",
            "10485760".to_string(), // 10 MB
        );
        let config = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap();
        assert_eq!(config.default_max_file_size_bytes, 10_485_760);
    }

    #[test]
    fn test_rejects_zero_max_file_size() {
        let mut map = sample_env_map();
        map.insert("DEFAULT_MAX_FILE_SIZE_BYTES", "0".to_string());
        let err = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { key, .. } if key == "DEFAULT_MAX_FILE_SIZE_BYTES")
        );
    }

    #[test]
    fn test_custom_ignore_patterns() {
        let mut map = sample_env_map();
        map.insert(
            "DEFAULT_IGNORE_PATTERNS",
            ".git, node_modules, temp, .cache".to_string(),
        );
        let config = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap();
        assert_eq!(
            config.default_ignore_patterns,
            vec![".git", "node_modules", "temp", ".cache"]
        );
    }

    #[test]
    fn test_custom_bm25_weights() {
        let mut map = sample_env_map();
        map.insert("DEFAULT_BM25_TITLE_WEIGHT", "5.0".to_string());
        map.insert("DEFAULT_BM25_TAGS_WEIGHT", "3.5".to_string());
        map.insert("DEFAULT_BM25_CONTENT_WEIGHT", "1.5".to_string());

        let config = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap();
        assert_eq!(
            config.default_bm25_weights,
            Bm25Weights {
                title: 5.0,
                tags: 3.5,
                content: 1.5,
            }
        );
    }

    #[test]
    fn test_rejects_negative_and_zero_bm25_weights() {
        let mut map_neg = sample_env_map();
        map_neg.insert("DEFAULT_BM25_TITLE_WEIGHT", "-1.0".to_string());
        let err_neg = AppConfig::from_lookup(|k| map_neg.get(k).cloned()).unwrap_err();
        assert!(
            matches!(err_neg, ConfigError::InvalidValue { key, .. } if key == "DEFAULT_BM25_TITLE_WEIGHT")
        );

        let mut map_zero = sample_env_map();
        map_zero.insert("DEFAULT_BM25_TAGS_WEIGHT", "0.0".to_string());
        let err_zero = AppConfig::from_lookup(|k| map_zero.get(k).cloned()).unwrap_err();
        assert!(
            matches!(err_zero, ConfigError::InvalidValue { key, .. } if key == "DEFAULT_BM25_TAGS_WEIGHT")
        );
    }

    #[test]
    fn test_custom_concurrency_limit() {
        let mut map = sample_env_map();
        map.insert("FILE_READ_CONCURRENCY_LIMIT", "32".to_string());
        let config = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap();
        assert_eq!(config.file_read_concurrency_limit, 32);
    }

    #[test]
    fn test_rejects_zero_concurrency_limit() {
        let mut map = sample_env_map();
        map.insert("FILE_READ_CONCURRENCY_LIMIT", "0".to_string());
        let err = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { key, .. } if key == "FILE_READ_CONCURRENCY_LIMIT")
        );
    }
}
