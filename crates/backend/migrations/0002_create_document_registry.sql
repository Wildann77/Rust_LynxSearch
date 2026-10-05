-- Migration 0002: Document Registry
CREATE TABLE IF NOT EXISTS document_registry (
    id UUID PRIMARY KEY,
    folder_id UUID NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    relative_path TEXT NOT NULL,
    content_hash VARCHAR(64),
    file_size_bytes BIGINT NOT NULL,
    modified_at TIMESTAMPTZ NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'INDEXED',
    status_reason TEXT,
    last_indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_folder_relative_path UNIQUE (folder_id, relative_path),
    CONSTRAINT chk_document_registry_status CHECK (status IN ('INDEXED', 'SKIPPED', 'FAILED', 'EXCLUDED'))
);

CREATE INDEX IF NOT EXISTS idx_doc_registry_folder_id ON document_registry(folder_id);
CREATE INDEX IF NOT EXISTS idx_doc_registry_status ON document_registry(status);
