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
    // TODO(anh): consider a separate field for completed messages. Or maybe a list of in messages per state with reference to the messages?
    queue: Vec<Message>,
}

#[derive(Debug, PartialEq)]
enum MessageState {
    Pending,
    Reserved,
    Completed,
}

pub struct Message {
    pub id: Uuid,
    pub payload: Vec<u8>,
    pub tries: u32,
    state: MessageState,
}

impl QoxideQueue {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    pub fn size(&self) -> usize {
        self.queue.len()
    }

    pub fn insert(&mut self, payload: Vec<u8>) -> Uuid {
        let id = Uuid::new_v4();
        let message = Message {
            id,
            payload,
            tries: 0,
            state: MessageState::Pending,
        };
        self.queue.push(message);
        id
    }

    pub fn reserve(&mut self) -> Option<&Message> {
        // TODO(anh): this scales with size
        let message = self
            .queue
            .iter_mut()
            .find(|m| m.state == MessageState::Pending)?;
        message.state = MessageState::Reserved;
        message.tries += 1;
        Some(message)
    }

    pub fn complete(&mut self, id: Uuid) -> bool {
        let message = self.queue.iter_mut().find(|m| m.id == id);
        if let Some(message) = message {
            message.state = MessageState::Completed;
            return true;
        }
        return false;
    }

    pub fn fail(&mut self, id: Uuid) -> bool {
        let message = self.queue.iter_mut().find(|m| m.id == id);
        if let Some(message) = message {
            message.state = MessageState::Pending;
            return true;
        }
        return false;
    }

    pub fn drop(&mut self, id: Uuid) {
        let position = self.queue.iter().position(|m| m.id == id);
        if let Some(position) = position {
            self.queue.swap_remove(position);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_size() {
        let mut queue = QoxideQueue::new();
        assert_eq!(queue.size(), 0);
        let payload = b"test".to_vec();
        queue.insert(payload.clone());
        assert_eq!(queue.size(), 1);
    }

    #[test]
    fn test_messages_can_be_inserted_and_retrieved() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        queue.insert(payload.clone());

        assert_eq!(queue.size(), 1);
    }

    #[test]
    fn test_messages_can_change_state() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        let id = queue.insert(payload.clone());

        let message = queue.reserve().expect("Message should be found");
        assert_eq!(message.id, id);
        assert_eq!(message.payload, payload);
        assert_eq!(message.state, MessageState::Reserved);
        assert_eq!(message.tries, 1);

        queue.fail(id);
        let message = queue.reserve().expect("Message should be found");
        assert_eq!(message.id, id);
        assert_eq!(message.payload, payload);
        assert_eq!(message.state, MessageState::Reserved);
        assert_eq!(message.tries, 2);

        let completed = queue.complete(id);
        assert!(completed);
    }

    #[test]
    fn test_messages_can_be_dropped() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        let id = queue.insert(payload.clone());

        assert!(queue.size() == 1);
        queue.drop(id);
        assert!(queue.size() == 0);
    }
}
