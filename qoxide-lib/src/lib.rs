pub mod cli;

pub use cli::run;

use uuid::Uuid;

/**
 * The core data structure of a queue including
 * - the queue itself
 * - metadata about the queue
 */
pub struct QoxideQueue {
    // TODO(anh): consider whether to split payload into a separate field from the queue itself
    queue: Vec<Message>,
}

pub struct Message {
    id: Uuid,
    payload: Vec<u8>,
}

impl QoxideQueue {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    pub fn insert(&mut self, payload: Vec<u8>) -> Uuid {
        let id = Uuid::new_v4();
        let message = Message { id, payload };
        self.queue.push(message);
        id
    }

    pub fn get(&self, id: Uuid) -> Option<&Message> {
        self.queue.iter().find(|m| m.id == id)
    }

    pub fn remove(&mut self, id: Uuid) {
        self.queue.retain(|m| m.id != id);
    }

    pub fn pop(&mut self) -> Option<Message> {
        self.queue.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messages_can_be_inserted_and_retrieved() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        let id = queue.insert(payload.clone());

        let message = queue.get(id).expect("Message should be found");
        assert_eq!(message.id, id);
        assert_eq!(message.payload, payload);
    }

    #[test]
    fn test_messages_can_be_removed() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        let id = queue.insert(payload.clone());

        let message = queue.get(id).expect("Message should be found");
        assert_eq!(message.id, id);
        assert_eq!(message.payload, payload);

        queue.remove(id);
        let message = queue.get(id);
        assert!(message.is_none(), "Message should have been removed");
    }

    #[test]
    fn test_messages_can_be_popped() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        let id = queue.insert(payload.clone());

        let message = queue.pop().expect("Message should be found");
        assert_eq!(message.id, id);
        assert_eq!(message.payload, payload);
    }
}
