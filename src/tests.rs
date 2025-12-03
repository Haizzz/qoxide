use super::*;

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
        assert_eq!(queue.size().unwrap().pending, 1);
        queue.reserve().expect("Message should be found");
        assert_eq!(queue.size().unwrap().pending, 0);
    }

    #[test]
    fn test_fail_moves_to_dlq() {
        // max_attempts(3) means the job can run at most 3 times
        let mut queue = QoxideQueue::builder().max_attempts(3).build().unwrap();
        let payload = b"test".to_vec();
        queue.add(payload.clone()).expect("Failed to add message");

        let (id, _) = queue.reserve().expect("Message should be found");

        // First two failures should return to pending (attempts 1 and 2)
        let state = queue.fail(id).unwrap();
        assert_eq!(state, MessageState::Pending);
        assert_eq!(queue.size().unwrap().pending, 1);

        queue.reserve().unwrap();
        let state = queue.fail(id).unwrap();
        assert_eq!(state, MessageState::Pending);

        // Third failure should move to DLQ (attempt 3 = max_attempts)
        queue.reserve().unwrap();
        let state = queue.fail(id).unwrap();
        assert_eq!(state, MessageState::Dead);

        let sizes = queue.size().unwrap();
        assert_eq!(sizes.pending, 0);
        assert_eq!(sizes.dead, 1);
    }

    #[test]
    fn test_dead_letters() {
        // max_attempts(1) means the job can only run once
        let mut queue = QoxideQueue::builder().max_attempts(1).build().unwrap();
        let payload = b"dead message".to_vec();
        queue.add(payload.clone()).expect("Failed to add message");

        let (id, _) = queue.reserve().unwrap();

        // First failure moves to DLQ (max_attempts = 1)
        queue.fail(id).unwrap();

        let dead = queue.dead_letters().unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(queue.get(dead[0]).unwrap(), payload);
    }

    #[test]
    fn test_requeue_dead_letters() {
        let mut queue = QoxideQueue::builder().max_attempts(1).build().unwrap();
        let id1 = queue.add(b"test1".to_vec()).unwrap();
        let id2 = queue.add(b"test2".to_vec()).unwrap();

        queue.reserve().unwrap();
        queue.reserve().unwrap();
        queue.fail(id1).unwrap();
        queue.fail(id2).unwrap();

        assert_eq!(queue.size().unwrap().dead, 2);

        queue.requeue_dead_letters(&[id1, id2]).unwrap();

        let sizes = queue.size().unwrap();
        assert_eq!(sizes.dead, 0);
        assert_eq!(sizes.pending, 2);
    }

    #[test]
    fn test_remove() {
        let mut queue = QoxideQueue::new();
        let id = queue.add(b"test".to_vec()).unwrap();

        assert_eq!(queue.size().unwrap().total, 1);

        queue.remove(id).unwrap();

        assert_eq!(queue.size().unwrap().total, 0);
    }
}
