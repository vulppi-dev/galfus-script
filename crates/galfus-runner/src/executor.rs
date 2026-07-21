use galfus_contract::{RunnableTask, ThreadExecutor, ThreadResult};
use std::collections::VecDeque;
use std::sync::Mutex;

pub struct SingleThreadExecutor {
    queue: Mutex<VecDeque<Box<dyn RunnableTask>>>,
}

impl SingleThreadExecutor {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn run_until_idle(&self) -> Result<(), String> {
        loop {
            let task = {
                let mut q = self.queue.lock().unwrap();
                q.pop_front()
            };

            let Some(task) = task else {
                break;
            };

            match task.run(100) {
                ThreadResult::Yielded(t) => {
                    self.queue.lock().unwrap().push_back(t);
                }
                ThreadResult::Blocked => {
                    // It will be re-spawned when unblocked by another thread sending a message.
                    // For a single threaded executor, if no other threads are running, it is a deadlock.
                    // But maybe another thread is already in the queue.
                }
                ThreadResult::Completed(_code) => {
                    // Task finished
                }
                ThreadResult::Failed(err) => {
                    return Err(err);
                }
            }
        }
        Ok(())
    }
}

impl ThreadExecutor for SingleThreadExecutor {
    fn spawn(&self, task: Box<dyn RunnableTask>) {
        self.queue.lock().unwrap().push_back(task);
    }
}

impl Default for SingleThreadExecutor {
    fn default() -> Self {
        Self::new()
    }
}
