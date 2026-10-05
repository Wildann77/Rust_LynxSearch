use crate::config::{AppConfig, Bm25Weights};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub max_file_size_bytes: u64,
    pub weights: Bm25Weights,
    pub ignore_patterns: Vec<String>,
}

impl AppSettings {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            max_file_size_bytes: config.default_max_file_size_bytes,
            weights: config.default_bm25_weights,
            ignore_patterns: config.default_ignore_patterns.clone(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            max_file_size_bytes: 2 * 1024 * 1024,
            weights: Bm25Weights {
                title: 3.0,
                tags: 2.0,
                content: 1.0,
            },
            ignore_patterns: vec![
                ".git".into(),
                "node_modules".into(),
                "target".into(),
                "dist".into(),
                "build".into(),
            ],
        }
    }
}
