-- Migration 0003: Indexing Jobs
CREATE TABLE IF NOT EXISTS indexing_jobs (
    id UUID PRIMARY KEY,
    folder_id UUID REFERENCES folders(id) ON DELETE CASCADE,
    job_type VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL,
    files_total INT NOT NULL DEFAULT 0,
    files_added INT NOT NULL DEFAULT 0,
    files_updated INT NOT NULL DEFAULT 0,
    files_deleted INT NOT NULL DEFAULT 0,
    files_skipped INT NOT NULL DEFAULT 0,
    files_failed INT NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_summary TEXT,
    CONSTRAINT chk_indexing_jobs_job_type CHECK (job_type IN ('IMPORT', 'RESCAN', 'REBUILD')),
    CONSTRAINT chk_indexing_jobs_status CHECK (status IN ('PENDING', 'RUNNING', 'COMPLETED', 'FAILED', 'CANCELLED'))
);

CREATE INDEX IF NOT EXISTS idx_jobs_folder_id ON indexing_jobs(folder_id);
CREATE INDEX IF NOT EXISTS idx_jobs_status ON indexing_jobs(status);
