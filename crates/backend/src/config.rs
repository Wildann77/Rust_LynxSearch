use std::env;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub database_url: String,
    pub elasticsearch_url: String,
    pub elasticsearch_index_alias: String,
    pub backend_bind_addr: String,
    pub backend_host: String,
    pub backend_port: u16,
    pub rust_log: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let _ = dotenvy::dotenv();
        Self::from_lookup(|k| env::var(k).ok())
    }

    pub fn from_lookup<F>(lookup: F) -> Result<Self, String>
    where
        F: Fn(&str) -> Option<String>,
    {
        let database_url = lookup("DATABASE_URL")
            .unwrap_or_else(|| "postgres://lynx:lynxpass@127.0.0.1:5432/lynxsearch".to_string());
        let elasticsearch_url =
            lookup("ELASTICSEARCH_URL").unwrap_or_else(|| "http://127.0.0.1:9200".to_string());
        let elasticsearch_index_alias =
            lookup("ELASTICSEARCH_INDEX_ALIAS").unwrap_or_else(|| "lynx_documents".to_string());

        let backend_host = lookup("BACKEND_HOST").unwrap_or_else(|| "127.0.0.1".to_string());
        let backend_port = lookup("BACKEND_PORT")
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(3001);

        let backend_bind_addr = lookup("BACKEND_BIND_ADDR")
            .unwrap_or_else(|| format!("{}:{}", backend_host, backend_port));

        if backend_bind_addr.starts_with("0.0.0.0") || backend_host == "0.0.0.0" {
            return Err("Security Violation: Binding to 0.0.0.0 is forbidden. Localhost binding only (127.0.0.1).".to_string());
        }

        let rust_log = lookup("RUST_LOG").unwrap_or_else(|| "info,backend=debug".to_string());

        Ok(Self {
            database_url,
            elasticsearch_url,
            elasticsearch_index_alias,
            backend_bind_addr,
            backend_host,
            backend_port,
            rust_log,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_default_config() {
        let empty_map = HashMap::<&str, String>::new();
        let config = AppConfig::from_lookup(|k| empty_map.get(k).cloned())
            .expect("Config should parse with defaults");

        assert_eq!(config.backend_bind_addr, "127.0.0.1:3001");
        assert_eq!(config.backend_host, "127.0.0.1");
        assert_eq!(config.backend_port, 3001);
        assert_eq!(
            config.database_url,
            "postgres://lynx:lynxpass@127.0.0.1:5432/lynxsearch"
        );
        assert_eq!(config.elasticsearch_url, "http://127.0.0.1:9200");
    }

    #[test]
    fn test_custom_bind_addr() {
        let mut map = HashMap::<&str, String>::new();
        map.insert("BACKEND_BIND_ADDR", "127.0.0.1:4000".to_string());
        let config = AppConfig::from_lookup(|k| map.get(k).cloned())
            .expect("Config should parse custom bind addr");

        assert_eq!(config.backend_bind_addr, "127.0.0.1:4000");
    }

    #[test]
    fn test_rejects_zero_bind_address() {
        let mut map = HashMap::<&str, String>::new();
        map.insert("BACKEND_BIND_ADDR", "0.0.0.0:3001".to_string());
        let err = AppConfig::from_lookup(|k| map.get(k).cloned()).unwrap_err();

        assert!(err.contains("Security Violation"));
    }
}
