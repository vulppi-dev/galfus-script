#[cfg(test)]
mod tests;

use galfus_contract::{RunnableTask, ThreadExecutor, ThreadResult};
use std::collections::VecDeque;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

pub struct SingleThreadExecutor {
    queue: Mutex<VecDeque<Box<dyn RunnableTask>>>,
    next_thread_id: AtomicU64,
}

impl SingleThreadExecutor {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            next_thread_id: AtomicU64::new(1),
        }
    }
}

impl ThreadExecutor for SingleThreadExecutor {
    fn allocate_thread_id(&self) -> u64 {
        self.next_thread_id.fetch_add(1, Ordering::Relaxed)
    }

    fn spawn(&self, task: Box<dyn RunnableTask>) {
        self.queue.lock().unwrap().push_back(task);
    }

    fn run_until_idle(&self) -> Result<i32, String> {
        let mut exit_code = 0;
        let mut pending_timeout = None;
        loop {
            let task = {
                let mut q = self.queue.lock().unwrap();
                q.pop_front()
            };

            let Some(task) = task else {
                let Some(timeout) = pending_timeout.take() else {
                    break;
                };
                std::thread::sleep(timeout);
                continue;
            };

            match task.run(100) {
                ThreadResult::Yielded(t) => {
                    self.queue.lock().unwrap().push_back(t);
                }
                ThreadResult::Blocked { timeout } => {
                    pending_timeout = match (pending_timeout, timeout) {
                        (Some(current), Some(next)) => Some(current.min(next)),
                        (Some(current), None) => Some(current),
                        (None, next) => next,
                    };
                }
                ThreadResult::Completed(code) => {
                    exit_code = code;
                }
                ThreadResult::Failed(err) => {
                    return Err(err);
                }
            }
        }
        Ok(exit_code)
    }
}

impl Default for SingleThreadExecutor {
    fn default() -> Self {
        Self::new()
    }
}
