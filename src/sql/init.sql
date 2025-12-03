CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    state TEXT NOT NULL,
    payload_id INTEGER NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (payload_id) REFERENCES payloads (id)
);

CREATE TABLE IF NOT EXISTS payloads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data BLOB NOT NULL
);

-- Index on state for efficient filtering and grouping
CREATE INDEX IF NOT EXISTS idx_messages_state ON messages(state);
