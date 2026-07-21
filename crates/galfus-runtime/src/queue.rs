use crate::registry::ThreadId;
use std::collections::{HashSet, VecDeque};

pub struct RunnableQueue {
    queue: VecDeque<ThreadId>,
}

impl RunnableQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, id: ThreadId) {
        self.queue.push_back(id);
    }

    pub fn dequeue(&mut self) -> Option<ThreadId> {
        self.queue.pop_front()
    }
}

impl Default for RunnableQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BlockedQueue {
    blocked: HashSet<ThreadId>,
}

impl BlockedQueue {
    pub fn new() -> Self {
        Self {
            blocked: HashSet::new(),
        }
    }

    pub fn block(&mut self, id: ThreadId) {
        self.blocked.insert(id);
    }

    pub fn unblock(&mut self, id: ThreadId) -> bool {
        self.blocked.remove(&id)
    }
}

impl Default for BlockedQueue {
    fn default() -> Self {
        Self::new()
    }
}
