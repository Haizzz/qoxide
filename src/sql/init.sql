CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    state TEXT NOT NULL,
    payload_id INTEGER NOT NULL,
    FOREIGN KEY (payload_id) REFERENCES payloads (id)
    -- TODO(anh): maybe created and updated timestamp
);

CREATE TABLE IF NOT EXISTS payloads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data BLOB NOT NULL
);

-- Index on state for efficient filtering and grouping
CREATE INDEX IF NOT EXISTS idx_messages_state ON messages(state);
