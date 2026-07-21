use galfus_vm::thread::VirtualThread;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadId(pub u64);

pub struct ThreadRegistry {
    threads: HashMap<ThreadId, VirtualThread>,
    next_id: u64,
}

impl ThreadRegistry {
    pub fn new() -> Self {
        Self {
            threads: HashMap::new(),
            next_id: 1, // Start at 1 to reserve 0 or just for clarity
        }
    }

    pub fn register(&mut self, thread: VirtualThread) -> ThreadId {
        let id = ThreadId(self.next_id);
        self.next_id += 1;
        self.threads.insert(id, thread);
        id
    }

    pub fn register_with_id(&mut self, id: ThreadId, thread: VirtualThread) {
        self.threads.insert(id, thread);
    }

    pub fn get_mut(&mut self, id: ThreadId) -> Option<&mut VirtualThread> {
        self.threads.get_mut(&id)
    }

    pub fn get(&self, id: ThreadId) -> Option<&VirtualThread> {
        self.threads.get(&id)
    }

    pub fn remove(&mut self, id: ThreadId) -> Option<VirtualThread> {
        self.threads.remove(&id)
    }
}

impl Default for ThreadRegistry {
    fn default() -> Self {
        Self::new()
    }
}
