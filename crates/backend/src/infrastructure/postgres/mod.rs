pub mod connection;
pub mod folder_repo;
pub mod job_repo;
pub mod registry_repo;
pub mod settings_repo;

pub use connection::{
    DEFAULT_ACQUIRE_TIMEOUT, DEFAULT_IDLE_TIMEOUT, DEFAULT_MAX_CONNECTIONS, DEFAULT_MAX_LIFETIME,
    DEFAULT_MIN_CONNECTIONS, build_pg_pool_options, check_database_readiness, create_pg_pool_eager,
    create_pg_pool_lazy, run_migrations,
};
pub use folder_repo::PgFolderRepository;
pub use job_repo::PgJobRepository;
pub use registry_repo::PgDocumentRegistryRepository;
pub use settings_repo::PgSettingsRepository;

use sqlx::migrate::Migrator;

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrator_contains_migration_0001() {
        let migrations = MIGRATOR.iter().collect::<Vec<_>>();
        assert!(!migrations.is_empty(), "Migrations should not be empty");

        let m1 = migrations
            .iter()
            .find(|m| m.version == 1)
            .expect("Migration 0001 (folders) must exist");

        assert_eq!(m1.migration_type, sqlx::migrate::MigrationType::Simple);
        assert!(
            m1.description.contains("folders"),
            "Description must reference folders"
        );

        let sql = &m1.sql;
        assert!(sql.contains("CREATE TABLE"), "Must contain CREATE TABLE");
        assert!(sql.contains("folders"), "Must create folders table");
        assert!(
            sql.contains("id UUID PRIMARY KEY"),
            "Must define id UUID PRIMARY KEY"
        );
        assert!(
            sql.contains("root_path TEXT NOT NULL UNIQUE"),
            "Must define root_path UNIQUE"
        );
        assert!(
            sql.contains("created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()"),
            "Must have created_at"
        );
        assert!(
            sql.contains("last_scanned_at TIMESTAMPTZ"),
            "Must have last_scanned_at"
        );
        assert!(
            sql.contains("status VARCHAR(32) NOT NULL DEFAULT 'IDLE'"),
            "Must have status default IDLE"
        );
        assert!(
            sql.contains("chk_folders_status")
                || sql.contains("CHECK (status IN ('IDLE', 'SCANNING', 'ERROR'))"),
            "Must restrict status to IDLE, SCANNING, ERROR"
        );
        assert!(
            sql.contains("CREATE INDEX") && sql.contains("idx_folders_root_path"),
            "Must create index on root_path"
        );
    }

    #[test]
    fn test_migrator_contains_migration_0002_document_registry() {
        let migrations = MIGRATOR.iter().collect::<Vec<_>>();
        let m2 = migrations
            .iter()
            .find(|m| m.version == 2)
            .expect("Migration 0002 (document_registry) must exist");

        assert_eq!(m2.migration_type, sqlx::migrate::MigrationType::Simple);
        assert!(
            m2.description.contains("document") && m2.description.contains("registry"),
            "Description must reference document and registry"
        );

        let sql = &m2.sql;
        assert!(sql.contains("CREATE TABLE"), "Must contain CREATE TABLE");
        assert!(
            sql.contains("document_registry"),
            "Must create document_registry table"
        );
        assert!(
            sql.contains("id UUID PRIMARY KEY"),
            "Must define id UUID PRIMARY KEY"
        );
        assert!(
            sql.contains("folder_id UUID NOT NULL REFERENCES folders(id) ON DELETE CASCADE"),
            "Must define folder_id foreign key with CASCADE DELETE"
        );
        assert!(
            sql.contains("relative_path TEXT NOT NULL"),
            "Must define relative_path"
        );
        assert!(
            sql.contains("content_hash VARCHAR(64)"),
            "Must define nullable content_hash"
        );
        assert!(
            sql.contains("file_size_bytes BIGINT NOT NULL"),
            "Must define file_size_bytes"
        );
        assert!(
            sql.contains("modified_at TIMESTAMPTZ NOT NULL"),
            "Must define modified_at"
        );
        assert!(
            sql.contains("status VARCHAR(32) NOT NULL DEFAULT 'INDEXED'"),
            "Must define status default INDEXED"
        );
        assert!(
            sql.contains("status_reason TEXT"),
            "Must define status_reason"
        );
        assert!(
            sql.contains("last_indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()"),
            "Must define last_indexed_at"
        );
        assert!(
            sql.contains("uq_folder_relative_path UNIQUE (folder_id, relative_path)"),
            "Must enforce composite unique constraint on (folder_id, relative_path)"
        );
        assert!(
            sql.contains("chk_document_registry_status")
                && sql.contains("CHECK (status IN ('INDEXED', 'SKIPPED', 'FAILED', 'EXCLUDED'))"),
            "Must constrain document status"
        );
        assert!(
            sql.contains("CREATE INDEX") && sql.contains("idx_doc_registry_folder_id"),
            "Must create index on folder_id"
        );
        assert!(
            sql.contains("CREATE INDEX") && sql.contains("idx_doc_registry_status"),
            "Must create index on status"
        );
    }

    #[test]
    fn test_migrator_contains_migration_0003_indexing_jobs() {
        let migrations = MIGRATOR.iter().collect::<Vec<_>>();
        let m3 = migrations
            .iter()
            .find(|m| m.version == 3)
            .expect("Migration 0003 (indexing_jobs) must exist");

        assert_eq!(m3.migration_type, sqlx::migrate::MigrationType::Simple);
        assert!(
            m3.description.contains("indexing") && m3.description.contains("jobs"),
            "Description must reference indexing and jobs"
        );

        let sql = &m3.sql;
        assert!(sql.contains("CREATE TABLE"), "Must contain CREATE TABLE");
        assert!(
            sql.contains("indexing_jobs"),
            "Must create indexing_jobs table"
        );
        assert!(
            sql.contains("id UUID PRIMARY KEY"),
            "Must define id UUID PRIMARY KEY"
        );
        assert!(
            sql.contains("folder_id UUID REFERENCES folders(id) ON DELETE CASCADE"),
            "Must define nullable folder_id foreign key with CASCADE DELETE"
        );
        assert!(
            sql.contains("job_type VARCHAR(32) NOT NULL"),
            "Must define job_type"
        );
        assert!(
            sql.contains("status VARCHAR(32) NOT NULL"),
            "Must define status"
        );
        assert!(
            sql.contains("files_total INT NOT NULL DEFAULT 0"),
            "Must define files_total"
        );
        assert!(
            sql.contains("files_added INT NOT NULL DEFAULT 0"),
            "Must define files_added"
        );
        assert!(
            sql.contains("files_updated INT NOT NULL DEFAULT 0"),
            "Must define files_updated"
        );
        assert!(
            sql.contains("files_deleted INT NOT NULL DEFAULT 0"),
            "Must define files_deleted"
        );
        assert!(
            sql.contains("files_skipped INT NOT NULL DEFAULT 0"),
            "Must define files_skipped"
        );
        assert!(
            sql.contains("files_failed INT NOT NULL DEFAULT 0"),
            "Must define files_failed"
        );
        assert!(
            sql.contains("started_at TIMESTAMPTZ NOT NULL DEFAULT NOW()"),
            "Must define started_at"
        );
        assert!(
            sql.contains("completed_at TIMESTAMPTZ"),
            "Must define completed_at"
        );
        assert!(
            sql.contains("error_summary TEXT"),
            "Must define error_summary"
        );
        assert!(
            sql.contains("chk_indexing_jobs_job_type")
                && sql.contains("CHECK (job_type IN ('IMPORT', 'RESCAN', 'REBUILD'))"),
            "Must constrain job_type"
        );
        assert!(
            sql.contains("chk_indexing_jobs_status")
                && sql.contains(
                    "CHECK (status IN ('PENDING', 'RUNNING', 'COMPLETED', 'FAILED', 'CANCELLED'))"
                ),
            "Must constrain job status"
        );
        assert!(
            sql.contains("CREATE INDEX") && sql.contains("idx_jobs_folder_id"),
            "Must create index on folder_id"
        );
        assert!(
            sql.contains("CREATE INDEX") && sql.contains("idx_jobs_status"),
            "Must create index on status"
        );
    }

    #[test]
    fn test_migrator_contains_migration_0004_settings() {
        let migrations = MIGRATOR.iter().collect::<Vec<_>>();
        let m4 = migrations
            .iter()
            .find(|m| m.version == 4)
            .expect("Migration 0004 (settings) must exist");

        assert_eq!(m4.migration_type, sqlx::migrate::MigrationType::Simple);
        assert!(
            m4.description.contains("settings"),
            "Description must reference settings"
        );

        let sql = &m4.sql;
        assert!(sql.contains("CREATE TABLE"), "Must contain CREATE TABLE");
        assert!(sql.contains("settings"), "Must create settings table");
        assert!(
            sql.contains("key VARCHAR(64) PRIMARY KEY"),
            "Must define key PK"
        );
        assert!(
            sql.contains("value JSONB NOT NULL"),
            "Must define value JSONB"
        );
        assert!(
            sql.contains("updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()"),
            "Must define updated_at"
        );
        assert!(
            sql.contains("INSERT INTO settings")
                && sql.contains("'app_settings'")
                && sql.contains("ON CONFLICT (key) DO NOTHING"),
            "Must seed default app_settings row idempotently"
        );
    }

    #[test]
    fn test_pg_repositories_implement_ports() {
        fn assert_folder_repo<T: crate::domain::ports::FolderRepository>() {}
        fn assert_registry_repo<T: crate::domain::ports::DocumentRegistryRepository>() {}
        fn assert_job_repo<T: crate::domain::ports::JobRepository>() {}
        fn assert_settings_repo<T: crate::domain::ports::SettingsRepository>() {}

        assert_folder_repo::<PgFolderRepository>();
        assert_registry_repo::<PgDocumentRegistryRepository>();
        assert_job_repo::<PgJobRepository>();
        assert_settings_repo::<PgSettingsRepository>();
    }
}
