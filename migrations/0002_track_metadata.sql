-- Keep metadata read from audio files separate from filesystem identity and lyrics workflow state
CREATE TABLE IF NOT EXISTS track_metadata (
    track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    title TEXT,
    album TEXT,
    duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
    file_format TEXT NOT NULL CHECK (file_format IN ('flac', 'mp3', 'opus', 'ogg', 'm4a')),
    read_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Artists and genres may contain multiple ordered values. Keeping them as
-- rows avoids lossy delimiter-based storage and supports future tag repairs.
CREATE TABLE IF NOT EXISTS track_metadata_values (
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    field TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0),
    value TEXT NOT NULL CHECK (length(trim(value)) > 0),
    PRIMARY KEY (track_id, field, position)
);

CREATE INDEX IF NOT EXISTS track_metadata_values_field_value_idx
    ON track_metadata_values(field, value);

CREATE TABLE IF NOT EXISTS track_metadata_issues (
    id INTEGER PRIMARY KEY,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    field TEXT,
    kind TEXT NOT NULL CHECK (kind IN ('missing', 'invalid', 'read_error')),
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS track_metadata_issues_track_id_idx
    ON track_metadata_issues(track_id);
