//! A lightweight local job queue backed by SQLite.
//!
//! # Example
//!
//! ```
//! use qoxide::QoxideQueue;
//!
//! # fn main() -> Result<(), rusqlite::Error> {
//! let mut queue = QoxideQueue::builder()
//!     .path(":memory:")  // optional: persists to file
//!     .max_attempts(3)   // optional: moves to DLQ after 3 failed attempts
//!     .build()?;
//!
//! // Add and process a message
//! let id = queue.add(b"job payload".to_vec())?;
//! let (id, payload) = queue.reserve()?;
//! queue.complete(id)?;
//!
//! // Or fail and retry
//! let id = queue.add(b"another job".to_vec())?;
//! let (id, _) = queue.reserve()?;
//! queue.fail(id)?;
//!
//! // Inspect and manage dead letters
//! let dead_ids = queue.dead_letters()?;
//! queue.requeue_dead_letters(&dead_ids)?;
//! # Ok(())
//! # }
//! ```

use rusqlite::{Connection, Error, params};

/// A SQLite-backed message queue.
///
/// Messages flow through states: `Pending` → `Reserved` → `Completed` (or `Dead`).
///
/// Use [`add`](Self::add) to enqueue, [`reserve`](Self::reserve) to dequeue,
/// and [`complete`](Self::complete) or [`fail`](Self::fail) to finish processing.
///
/// Optionally configure a max attempts limit to move failed messages
/// to the dead letter queue after N attempts.
pub struct QoxideQueue {
    db: Connection,
    max_attempts: Option<u32>,
}

/// The state of a message in the queue.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MessageState {
    /// Message is waiting to be processed.
    Pending,
    /// Message is currently being processed by a worker.
    Reserved,
    /// Message has been successfully processed.
    Completed,
    /// Message has exceeded max attempts and is in the dead letter queue.
    Dead,
}

impl MessageState {
    /// Returns the string representation of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageState::Pending => "PENDING",
            MessageState::Reserved => "RESERVED",
            MessageState::Completed => "COMPLETED",
            MessageState::Dead => "DEAD",
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
    /// Number of messages in the dead letter queue.
    pub dead: usize,
}

/// Builder for creating a [`QoxideQueue`] with custom configuration.
///
/// # Example
///
/// ```
/// use qoxide::QoxideQueueBuilder;
///
/// let queue = QoxideQueueBuilder::new()
///     .path(":memory:")
///     .max_attempts(3)
///     .build();
/// ```
#[derive(Default)]
pub struct QoxideQueueBuilder {
    path: Option<String>,
    max_attempts: Option<u32>,
}

impl QoxideQueueBuilder {
    /// Creates a new builder with default settings (in-memory, unlimited attempts).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the database path for persistence.
    ///
    /// If not set, the queue will be in-memory.
    pub fn path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    /// Sets the maximum number of attempts before moving to dead letter queue.
    ///
    /// For example, `max_attempts(3)` means a job can run at most 3 times.
    /// If all 3 attempts fail, the message moves to the dead letter queue.
    ///
    /// If not set, messages can be retried indefinitely.
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = Some(max_attempts);
        self
    }

    /// Builds the queue with the configured settings.
    pub fn build(self) -> Result<QoxideQueue, Error> {
        let path = self.path.as_deref().unwrap_or(":memory:");
        let db = Connection::open(path)?;
        let queue = QoxideQueue {
            db,
            max_attempts: self.max_attempts,
        };
        queue.init(path)?;
        Ok(queue)
    }
}

impl Default for QoxideQueue {
    fn default() -> Self {
        Self::builder().build().unwrap()
    }
}

impl QoxideQueue {
    /// Creates a new in-memory queue with unlimited attempts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a builder for creating a queue with custom configuration.
    ///
    /// # Example
    ///
    /// ```
    /// use qoxide::QoxideQueue;
    ///
    /// let queue = QoxideQueue::builder()
    ///     .path(":memory:")
    ///     .max_attempts(3)
    ///     .build();
    /// ```
    pub fn builder() -> QoxideQueueBuilder {
        QoxideQueueBuilder::new()
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
            dead: 0,
        };
        while let Some(row) = rows.next()? {
            let state: String = row.get(0)?;
            let count = row.get(1)?;
            total += count;
            match state.as_str() {
                "PENDING" => sizes.pending = count,
                "RESERVED" => sizes.reserved = count,
                "COMPLETED" => sizes.completed = count,
                "DEAD" => sizes.dead = count,
                _ => (),
            }
        }
        sizes.total = total;

        Ok(sizes)
    }

    /// Returns the payload for a message by ID.
    pub fn get(&self, id: i64) -> Result<Vec<u8>, Error> {
        self.db.query_row(
            "SELECT p.data FROM messages m JOIN payloads p ON m.payload_id = p.id WHERE m.id = ?",
            params![id],
            |row| row.get(0),
        )
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

    /// Marks a reserved message as failed.
    ///
    /// If the queue has no max attempts, the message returns to pending state.
    /// If the queue has a max attempts limit and this was the final attempt,
    /// the message moves to the dead letter queue.
    ///
    /// Returns the new state of the message.
    pub fn fail(&mut self, id: i64) -> Result<MessageState, Error> {
        let new_state = match self.max_attempts {
            None => MessageState::Pending,
            Some(max) => {
                let attempt_count: u32 = self.db.query_row(
                    "SELECT attempt_count FROM messages WHERE id = ?",
                    params![id],
                    |row| row.get::<_, u32>(0).map(|c| c + 1),
                )?;
                if attempt_count >= max {
                    MessageState::Dead
                } else {
                    MessageState::Pending
                }
            }
        };

        self.db.execute(
            "UPDATE messages SET state = ?, attempt_count = attempt_count + 1 WHERE id = ?",
            params![new_state.as_str(), id],
        )?;

        Ok(new_state)
    }

    /// Removes a message by ID permanently.
    pub fn remove(&mut self, id: i64) -> Result<(), Error> {
        self.db
            .execute("DELETE FROM messages WHERE id = ?", params![id])?;
        Ok(())
    }

    /// Returns the IDs of all messages in the dead letter queue.
    pub fn dead_letters(&self) -> Result<Vec<i64>, Error> {
        let mut statement = self
            .db
            .prepare_cached("SELECT id FROM messages WHERE state = 'DEAD'")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    /// Requeues dead letter messages back to pending state, resetting their attempt counts.
    pub fn requeue_dead_letters(&mut self, ids: &[i64]) -> Result<(), Error> {
        let placeholders: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "UPDATE messages SET state = 'PENDING', attempt_count = 0 WHERE id IN ({}) AND state = 'DEAD'",
            placeholders
        );
        self.db.execute(&sql, rusqlite::params_from_iter(ids))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
