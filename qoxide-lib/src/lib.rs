use rusqlite::{Connection, Error, params};

/**
 * The core data structure of a queue including
 * - the queue itself
 * - metadata about the queue
 */
pub struct QoxideQueue {
    db: Connection,
}

#[derive(Debug, PartialEq)]
pub enum MessageState {
    Pending,
    Reserved,
    Completed,
}

impl MessageState {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "PENDING" => Ok(MessageState::Pending),
            "RESERVED" => Ok(MessageState::Reserved),
            "COMPLETED" => Ok(MessageState::Completed),
            _ => Err(format!("Invalid state: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MessageState::Pending => "PENDING",
            MessageState::Reserved => "RESERVED",
            MessageState::Completed => "COMPLETED",
        }
    }
}

#[derive(Debug)]
pub struct QueueSize {
    total: usize,
    pending: usize,
    reserved: usize,
    completed: usize,
}

impl QoxideQueue {
    pub fn new() -> Self {
        Self::new_with_path(":memory:")
    }

    pub fn new_with_path(path: &str) -> Self {
        let db = Connection::open(path).expect("Failed to open database connection");
        let queue = Self { db };
        queue.init().expect("Failed to initialize database");

        queue
    }

    fn init(&self) -> Result<(), Error> {
        let init_schema_sql = include_str!("sql/init.sql");
        self.db.execute_batch(init_schema_sql)
    }

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

    pub fn reserve(&mut self) -> Result<(i64, Vec<u8>), Error> {
        self.db
            .query_one(include_str!("sql/reserve.sql"), [], |row| {
                let id: i64 = row.get(0)?;
                let payload: Vec<u8> = row.get(1)?;
                Ok((id, payload))
            })
    }

    pub fn complete(&self, id: i64) -> Result<(), Error> {
        self.db.execute(
            include_str!("sql/set_message_state.sql"),
            params![MessageState::Completed.as_str(), id],
        )?;
        Ok(())
    }

    pub fn fail(&mut self, id: i64) -> Result<(), Error> {
        self.db.execute(
            include_str!("sql/set_message_state.sql"),
            params![MessageState::Pending.as_str(), id],
        )?;
        Ok(())
    }

    // TODO(anh): add method to drop and clean up queue and indices
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
        queue.add(payload.clone());

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
