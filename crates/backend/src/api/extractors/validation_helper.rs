use std::borrow::Cow;
use validator::{ValidationErrors, ValidationErrorsKind};

/// Konversi terstruktur dari `validator::ValidationErrors` ke `serde_json::Value`.
/// Format sesuai ARCHITECTURE.md: `{"field": ["pesan error", ...]}`.
pub fn validation_errors_to_json(errors: &ValidationErrors) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    for (field, kind) in errors.errors() {
        match kind {
            ValidationErrorsKind::Field(field_errors) => {
                let msgs: Vec<serde_json::Value> = field_errors
                    .iter()
                    .map(|err| {
                        let msg = err
                            .message
                            .as_ref()
                            .map(Cow::to_string)
                            .unwrap_or_else(|| err.code.to_string());
                        serde_json::Value::String(msg)
                    })
                    .collect();
                map.insert(field.to_string(), serde_json::Value::Array(msgs));
            }
            ValidationErrorsKind::Struct(nested_errors) => {
                map.insert(field.to_string(), validation_errors_to_json(nested_errors));
            }
            ValidationErrorsKind::List(list_errors) => {
                let mut list_map = serde_json::Map::new();
                for (index, item_errors) in list_errors {
                    list_map.insert(index.to_string(), validation_errors_to_json(item_errors));
                }
                map.insert(field.to_string(), serde_json::Value::Object(list_map));
            }
        }
    }

    if map.is_empty() {
        let mut fallback = serde_json::Map::new();
        fallback.insert(
            "_".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::String(
                "Validation failed".to_string(),
            )]),
        );
        serde_json::Value::Object(fallback)
    } else {
        serde_json::Value::Object(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[derive(Validate)]
    struct SampleDto {
        #[validate(range(min = 1, max = 100, message = "Ukuran harus 1-100"))]
        size: u32,
    }

    #[test]
    fn test_validation_errors_to_json_format() {
        let sample = SampleDto { size: 999 };
        let errs = sample.validate().unwrap_err();
        let json = validation_errors_to_json(&errs);

        assert_eq!(
            json,
            serde_json::json!({
                "size": ["Ukuran harus 1-100"]
            })
        );
    }
}
