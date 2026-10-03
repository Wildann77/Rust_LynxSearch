use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LivenessResponseDto {
    pub status: String,
}

impl Default for LivenessResponseDto {
    fn default() -> Self {
        Self {
            status: "alive".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentHealthDto {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadinessResponseDto {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<ComponentHealthDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elasticsearch: Option<ComponentHealthDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthSummaryResponseDto {
    pub status: String,
    pub version: String,
    pub timestamp: String,
    pub database: ComponentHealthDto,
    pub elasticsearch: ComponentHealthDto,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liveness_dto_serialization() {
        let dto = LivenessResponseDto::default();
        let json = serde_json::to_string(&dto).unwrap();
        assert_eq!(json, r#"{"status":"alive"}"#);
    }

    #[test]
    fn test_readiness_dto_serialization_ready() {
        let dto = ReadinessResponseDto {
            status: "ready".to_string(),
            database: None,
            elasticsearch: None,
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert_eq!(json, r#"{"status":"ready"}"#);
    }

    #[test]
    fn test_readiness_dto_serialization_unavailable() {
        let dto = ReadinessResponseDto {
            status: "unavailable".to_string(),
            database: Some(ComponentHealthDto {
                status: "down".to_string(),
                latency_ms: None,
                error: Some("connection refused".to_string()),
            }),
            elasticsearch: Some(ComponentHealthDto {
                status: "up".to_string(),
                latency_ms: Some(4),
                error: None,
            }),
        };
        let val: serde_json::Value = serde_json::to_value(&dto).unwrap();
        assert_eq!(val["status"], "unavailable");
        assert_eq!(val["database"]["status"], "down");
        assert_eq!(val["elasticsearch"]["status"], "up");
        assert_eq!(val["elasticsearch"]["latency_ms"], 4);
    }

    #[test]
    fn test_health_summary_dto_serialization() {
        let dto = HealthSummaryResponseDto {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
            timestamp: "2026-10-02T12:00:00Z".to_string(),
            database: ComponentHealthDto {
                status: "up".to_string(),
                latency_ms: Some(3),
                error: None,
            },
            elasticsearch: ComponentHealthDto {
                status: "up".to_string(),
                latency_ms: Some(5),
                error: None,
            },
        };
        let val: serde_json::Value = serde_json::to_value(&dto).unwrap();
        assert_eq!(val["status"], "ok");
        assert_eq!(val["version"], "1.0.0");
        assert_eq!(val["database"]["status"], "up");
        assert_eq!(val["database"]["latency_ms"], 3);
        assert_eq!(val["elasticsearch"]["status"], "up");
        assert_eq!(val["elasticsearch"]["latency_ms"], 5);
    }
}
