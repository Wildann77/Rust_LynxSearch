use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
pub struct UpdateBm25WeightsDto {
    #[validate(range(
        min = 0.1,
        max = 100.0,
        message = "Bobot title harus antara 0.1 dan 100.0"
    ))]
    pub title: Option<f32>,

    #[validate(range(
        min = 0.1,
        max = 100.0,
        message = "Bobot tags harus antara 0.1 dan 100.0"
    ))]
    pub tags: Option<f32>,

    #[validate(range(
        min = 0.1,
        max = 100.0,
        message = "Bobot content harus antara 0.1 dan 100.0"
    ))]
    pub content: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
pub struct UpdateSettingsRequestDto {
    #[validate(range(
        min = 1024,
        max = 104857600,
        message = "Ukuran file maksimum antara 1KB dan 100MB"
    ))]
    pub max_file_size_bytes: Option<u64>,

    #[validate(nested)]
    pub weights: Option<UpdateBm25WeightsDto>,

    pub ignore_patterns: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_settings_update() {
        let dto = UpdateSettingsRequestDto {
            max_file_size_bytes: Some(4 * 1024 * 1024),
            weights: Some(UpdateBm25WeightsDto {
                title: Some(4.0),
                tags: Some(2.5),
                content: Some(1.0),
            }),
            ignore_patterns: Some(vec![".git".into(), "node_modules".into()]),
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn test_invalid_settings_weight_range() {
        let dto = UpdateSettingsRequestDto {
            max_file_size_bytes: None,
            weights: Some(UpdateBm25WeightsDto {
                title: Some(0.0),
                tags: None,
                content: None,
            }),
            ignore_patterns: None,
        };
        assert!(dto.validate().is_err());
    }
}
