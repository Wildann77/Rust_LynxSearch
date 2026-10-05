-- Migration 0001: Folders
CREATE TABLE IF NOT EXISTS folders (
    id UUID PRIMARY KEY,
    root_path TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_scanned_at TIMESTAMPTZ,
    status VARCHAR(32) NOT NULL DEFAULT 'IDLE',
    CONSTRAINT chk_folders_status CHECK (status IN ('IDLE', 'SCANNING', 'ERROR'))
);

CREATE INDEX IF NOT EXISTS idx_folders_root_path ON folders(root_path);
