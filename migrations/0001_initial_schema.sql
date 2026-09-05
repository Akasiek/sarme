PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS tracks (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    file_size INTEGER NOT NULL CHECK (file_size >= 0),
    modified_at TEXT NOT NULL,
    content_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS scans (
    id INTEGER PRIMARY KEY,
    started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    finished_at TEXT,
    status TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'completed', 'completed_with_errors', 'failed')),
    discovered_tracks INTEGER NOT NULL DEFAULT 0 CHECK (discovered_tracks >= 0),
    processed_tracks INTEGER NOT NULL DEFAULT 0 CHECK (processed_tracks >= 0),
    error_count INTEGER NOT NULL DEFAULT 0 CHECK (error_count >= 0),
    error_message TEXT
);

CREATE TABLE IF NOT EXISTS scan_errors (
    id INTEGER PRIMARY KEY,
    scan_id INTEGER NOT NULL REFERENCES scans(id) ON DELETE CASCADE,
    track_id INTEGER REFERENCES tracks(id) ON DELETE SET NULL,
    path TEXT NOT NULL,
    error_message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Support scan history views and track-specific diagnostics.
CREATE INDEX IF NOT EXISTS scan_errors_scan_id_idx ON scan_errors(scan_id);
CREATE INDEX IF NOT EXISTS scan_errors_track_id_idx ON scan_errors(track_id);

CREATE TABLE IF NOT EXISTS lyric_lookup_attempts (
    id INTEGER PRIMARY KEY,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    attempted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    status TEXT NOT NULL CHECK (status IN ('matched', 'review', 'no_result', 'error')),
    query TEXT NOT NULL,
    candidate_count INTEGER NOT NULL DEFAULT 0 CHECK (candidate_count >= 0),
    error_message TEXT,
    next_retry_at TEXT,
    UNIQUE (id, track_id)
);

CREATE INDEX IF NOT EXISTS lyric_lookup_attempts_track_id_idx
    ON lyric_lookup_attempts(track_id);
-- Only scheduled retries need to participate in the retry queue index.
CREATE INDEX IF NOT EXISTS lyric_lookup_attempts_next_retry_at_idx
    ON lyric_lookup_attempts(next_retry_at)
    WHERE next_retry_at IS NOT NULL;

-- Temporarily keep full lyrics only for ambiguous results awaiting manual
-- review. Confident matches are written directly to a sidecar and not retained.
CREATE TABLE IF NOT EXISTS lyric_review_candidates (
    id INTEGER PRIMARY KEY,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    lookup_attempt_id INTEGER NOT NULL,
    provider TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    score INTEGER NOT NULL,
    synced_lyrics TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    -- Do not store the same provider result twice for one track.
    UNIQUE (track_id, provider, provider_id),
    -- A candidate cannot be attached to an attempt made for another track.
    FOREIGN KEY (lookup_attempt_id, track_id)
        REFERENCES lyric_lookup_attempts(id, track_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS lyric_review_candidates_track_id_idx
    ON lyric_review_candidates(track_id);

CREATE TABLE IF NOT EXISTS track_lyrics (
    track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN (
            'pending',
            'review',
            'no_result',
            'error',
            'written',
            'existing'
        )),
    sidecar_path TEXT,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
