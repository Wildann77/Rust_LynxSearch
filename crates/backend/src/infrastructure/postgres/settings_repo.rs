use crate::domain::models::AppSettings;
use crate::domain::ports::SettingsRepository;
use crate::error::AppError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};

const APP_SETTINGS_KEY: &str = "app_settings";

#[derive(Clone, Debug)]
pub struct PgSettingsRepository {
    pool: PgPool,
}

impl PgSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettingsRepository for PgSettingsRepository {
    async fn get_settings(&self) -> Result<AppSettings, AppError> {
        let row_opt = sqlx::query("SELECT value FROM settings WHERE key = $1")
            .bind(APP_SETTINGS_KEY)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::Database)?;

        match row_opt {
            Some(row) => {
                let value_json: serde_json::Value =
                    row.try_get("value").map_err(AppError::Database)?;
                serde_json::from_value(value_json).map_err(|e| {
                    AppError::Internal(format!("Failed to deserialize settings from DB: {e}"))
                })
            }
            None => {
                let default_settings = AppSettings::default();
                self.update_settings(&default_settings).await?;
                Ok(default_settings)
            }
        }
    }

    async fn update_settings(&self, settings: &AppSettings) -> Result<(), AppError> {
        let value_json = serde_json::to_value(settings)
            .map_err(|e| AppError::Internal(format!("Failed to serialize settings: {e}")))?;

        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) \
             VALUES ($1, $2, NOW()) \
             ON CONFLICT (key) DO UPDATE SET \
                 value = EXCLUDED.value, \
                 updated_at = NOW()",
        )
        .bind(APP_SETTINGS_KEY)
        .bind(value_json)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn reset_settings(&self) -> Result<AppSettings, AppError> {
        let default_settings = AppSettings::default();
        self.update_settings(&default_settings).await?;
        Ok(default_settings)
    }
}
