-- Migration 0004: Application Settings
CREATE TABLE IF NOT EXISTS settings (
    key VARCHAR(64) PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default settings row if not present
INSERT INTO settings (key, value, updated_at)
VALUES (
    'app_settings',
    json_build_object(
        'max_file_size_bytes', 2097152,
        'weights', json_build_object(
            'title', 3.0,
            'tags', 2.0,
            'content', 1.0
        ),
        'ignore_patterns', json_build_array(
            '.git',
            'node_modules',
            'target',
            'dist',
            'build'
        )
    ),
    NOW()
)
ON CONFLICT (key) DO NOTHING;
