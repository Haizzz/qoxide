pub mod cli;

use std::collections::HashMap;

pub use cli::run;

use uuid::Uuid;

/**
 * The core data structure of a queue including
 * - the queue itself
 * - metadata about the queue
 */
pub struct QoxideQueue {
    payloads: Vec<Vec<u8>>,
    queue: HashMap<Uuid, Message>,
    pending_ids: Vec<Uuid>,
}

#[derive(Debug, PartialEq)]
pub enum MessageState {
    Pending,
    Reserved,
    Completed,
}

#[derive(Debug)]
pub struct Message {
    payload_index: usize,
    pub tries: u32,
    pub state: MessageState,
}

impl QoxideQueue {
    pub fn new() -> Self {
        Self {
            payloads: Vec::new(),
            queue: HashMap::new(),
            pending_ids: Vec::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.queue.len()
    }

    pub fn insert(&mut self, payload: Vec<u8>) -> Uuid {
        let id = Uuid::new_v4();
        self.payloads.push(payload);
        let message = Message {
            payload_index: self.payloads.len() - 1,
            tries: 0,
            state: MessageState::Pending,
        };
        self.queue.insert(id, message);
        self.pending_ids.push(id);
        id
    }

    pub fn reserve(&mut self) -> Option<(Uuid, &Vec<u8>)> {
        let id = self.pending_ids.pop()?;
        let message = self.queue.get_mut(&id)?;
        message.state = MessageState::Reserved;
        message.tries += 1;

        let payload = &self.payloads[message.payload_index];
        Some((id, payload))
    }

    pub fn complete(&mut self, id: Uuid) -> bool {
        let message = self.queue.get_mut(&id);
        if let Some(message) = message {
            message.state = MessageState::Completed;
            true
        } else {
            false
        }
    }

    pub fn fail(&mut self, id: Uuid) -> bool {
        let message = self.queue.get_mut(&id);
        if let Some(message) = message {
            message.state = MessageState::Pending;
            self.pending_ids.push(id);
            true
        } else {
            false
        }
    }

    // TODO(anh): add method to drop and clean up queue and indices
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
    fn test_messages_can_be_inserted() {
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

        let payload = queue.reserve().expect("Message should be found");
        assert_eq!(payload, payload);
        assert_eq!(queue.pending_ids.len(), 0);
        assert_eq!(
            queue
                .queue
                .iter()
                .find(|(_, m)| m.state == MessageState::Reserved)
                .is_some(),
            true
        );

        queue.fail(id);
        assert_eq!(queue.pending_ids.len(), 1);
        assert_eq!(
            queue
                .queue
                .iter()
                .find(|(_, m)| m.state == MessageState::Reserved)
                .is_none(),
            true
        );

        queue.reserve().expect("Message should be found");
        assert_eq!(queue.pending_ids.len(), 0);
        assert_eq!(
            queue
                .queue
                .iter()
                .find(|(_, m)| m.state == MessageState::Reserved && m.tries == 2)
                .is_some(),
            true
        );

        assert!(queue.complete(id));
    }

    #[test]
    fn test_reserve_next_message() {
        let mut queue = QoxideQueue::new();
        let payload = b"test".to_vec();
        queue.insert(payload.clone());
        queue.insert(payload.clone());

        queue.reserve().expect("Message should be found");
        assert_eq!(queue.pending_ids.len(), 1);
        assert_eq!(
            queue
                .queue
                .iter()
                .filter(|(_, m)| m.state == MessageState::Reserved)
                .count(),
            1
        );

        queue.reserve().expect("Message should be found");
        assert_eq!(queue.pending_ids.len(), 0);
        assert_eq!(
            queue
                .queue
                .iter()
                .filter(|(_, m)| m.state == MessageState::Reserved)
                .count(),
            2
        );
    }
}
