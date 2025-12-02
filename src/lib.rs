//! A lightweight local job queue backed by SQLite.
//!
//! # Example
//!
//! ```
//! use qoxide::QoxideQueue;
//!
//! let mut queue = QoxideQueue::new();
//!
//! // Add a message
//! let id = queue.add(b"job payload".to_vec()).unwrap();
//!
//! // Reserve and process
//! let (id, payload) = queue.reserve().unwrap();
//! // ... process the job ...
//! queue.complete(id).unwrap();
//! ```

use rusqlite::{Connection, Error, params};

/// A SQLite-backed message queue.
///
/// Messages flow through states: `Pending` → `Reserved` → `Completed`.
/// Use [`add`](Self::add) to enqueue, [`reserve`](Self::reserve) to dequeue,
/// and [`complete`](Self::complete) or [`fail`](Self::fail) to finish processing.
pub struct QoxideQueue {
    db: Connection,
}

/// The state of a message in the queue.
#[derive(Debug, PartialEq)]
pub enum MessageState {
    /// Message is waiting to be processed.
    Pending,
    /// Message is currently being processed by a worker.
    Reserved,
    /// Message has been successfully processed.
    Completed,
}

impl MessageState {
    /// Returns the string representation of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageState::Pending => "PENDING",
            MessageState::Reserved => "RESERVED",
            MessageState::Completed => "COMPLETED",
        }
    }
}

/// A breakdown of message counts by state.
#[derive(Debug)]
pub struct QueueSize {
    /// Total number of messages in the queue.
    pub total: usize,
    /// Number of messages waiting to be processed.
    pub pending: usize,
    /// Number of messages currently being processed.
    pub reserved: usize,
    /// Number of successfully processed messages.
    pub completed: usize,
}

impl Default for QoxideQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl QoxideQueue {
    /// Creates a new in-memory queue.
    ///
    /// Data is lost when the queue is dropped. For persistence, use [`new_with_path`](Self::new_with_path).
    pub fn new() -> Self {
        Self::new_with_path(":memory:")
    }

    /// Creates a new file-backed queue at the given path.
    ///
    /// Enables WAL mode for better concurrent read performance.
    /// The database file is created if it doesn't exist.
    pub fn new_with_path(path: &str) -> Self {
        let db = Connection::open(path).expect("Failed to open database connection");
        let queue = Self { db };
        queue.init(path).expect("Failed to initialize database");

        queue
    }

    fn init(&self, path: &str) -> Result<(), Error> {
        if path != ":memory:" {
            self.db.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA busy_timeout=5000;",
            )?;
        }

        let init_schema_sql = include_str!("sql/init.sql");
        self.db.execute_batch(init_schema_sql)
    }

    /// Returns the count of messages in each state.
    pub fn size(&self) -> Result<QueueSize, Error> {
        let sql = include_str!("sql/get_size.sql");
        let mut statement = self.db.prepare_cached(sql)?;
        let mut rows = statement.query([])?;
        let mut total: usize = 0;
        let mut sizes = QueueSize {
            total: 0,
            pending: 0,
            reserved: 0,
            completed: 0,
        };
        while let Some(row) = rows.next()? {
            let state: String = row.get(0)?;
            let count = row.get(1)?;
            total += count;
            match state.as_str() {
                "PENDING" => sizes.pending = count,
                "RESERVED" => sizes.reserved = count,
                "COMPLETED" => sizes.completed = count,
                _ => (),
            }
        }
        sizes.total = total;

        Ok(sizes)
    }

    /// Adds a message to the queue with the given payload.
    ///
    /// Returns the message ID which can be used with [`complete`](Self::complete) or [`fail`](Self::fail).
    pub fn add(&mut self, payload: Vec<u8>) -> Result<i64, Error> {
        let transaction = self.db.transaction()?;
        transaction.execute("INSERT INTO payloads (data) VALUES (?);", params![&payload])?;
        let payload_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO messages (state, payload_id) VALUES (?, ?);",
            params![MessageState::Pending.as_str(), payload_id],
        )?;
        let message_id = transaction.last_insert_rowid();
        transaction.commit()?;
        Ok(message_id)
    }

    /// Atomically reserves the next pending message.
    ///
    /// Returns the message ID and payload. The message state changes from `Pending` to `Reserved`.
    /// Returns an error if no pending messages are available.
    pub fn reserve(&mut self) -> Result<(i64, Vec<u8>), Error> {
        self.db
            .query_one(include_str!("sql/reserve.sql"), [], |row| {
                let id: i64 = row.get(0)?;
                let payload: Vec<u8> = row.get(1)?;
                Ok((id, payload))
            })
    }

    /// Marks a reserved message as successfully completed.
    pub fn complete(&self, id: i64) -> Result<(), Error> {
        self.db.execute(
            include_str!("sql/set_message_state.sql"),
            params![MessageState::Completed.as_str(), id],
        )?;
        Ok(())
    }

    /// Marks a reserved message as failed, returning it to pending state.
    ///
    /// The message will be available for [`reserve`](Self::reserve) again.
    pub fn fail(&mut self, id: i64) -> Result<(), Error> {
        self.db.execute(
            include_str!("sql/set_message_state.sql"),
            params![MessageState::Pending.as_str(), id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_size() {
        let mut queue = QoxideQueue::new();
        let sizes = queue.size().expect("Failed to get queue size");
        assert_eq!(sizes.total, 0);
        assert_eq!(sizes.pending, 0);
        assert_eq!(sizes.reserved, 0);
        assert_eq!(sizes.completed, 0);

        let payload = b"test".to_vec();
        queue.add(payload.clone()).expect("Failed to add message");
        let sizes = queue.size().expect("Failed to get queue size");
        assert_eq!(sizes.total, 1);
        assert_eq!(sizes.pending, 1);
        assert_eq!(sizes.reserved, 0);
        assert_eq!(sizes.completed, 0);
    }

    #[test]
    fn test_messages_can_be_inserted() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        queue.add(payload.clone()).expect("Failed to add message");

        assert_eq!(queue.size().unwrap().pending, 1);
    }

    #[test]
    fn test_messages_can_change_state() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        let id = queue.add(payload.clone()).expect("Failed to add message");

        let payload = queue.reserve().expect("Message should be found");
        assert_eq!(payload, payload);
        assert_eq!(queue.size().unwrap().pending, 0);

        queue.fail(id).expect("Failed to fail message");
        assert_eq!(queue.size().unwrap().pending, 1);

        queue.reserve().expect("Message should be found");
        assert_eq!(queue.size().unwrap().pending, 0);
    }

    #[test]
    fn test_reserve_next_message() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        queue.add(payload.clone()).expect("Failed to add message");
        queue.add(payload.clone()).expect("Failed to add message");

        queue.reserve().expect("Message should be found");
        println!("queue.size().unwrap(): {:?}", queue.size().unwrap());
        assert_eq!(queue.size().unwrap().pending, 1);
        queue.reserve().expect("Message should be found");
        assert_eq!(queue.size().unwrap().pending, 0);
    }
}
