#[cfg(test)]
mod tests;

use galfus_contract::{ExecutorStepResult, RunnableTask, ThreadExecutor, ThreadResult};
use std::collections::VecDeque;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

pub struct SingleThreadExecutor {
    queue: Mutex<VecDeque<Box<dyn RunnableTask>>>,
    next_thread_id: AtomicU64,
    exit_code: std::sync::Mutex<i32>,
    exit_callback: Mutex<Option<Box<dyn Fn(Result<i32, String>) + Send + Sync>>>,
}

impl SingleThreadExecutor {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            next_thread_id: AtomicU64::new(1),
            exit_code: std::sync::Mutex::new(0),
            exit_callback: Mutex::new(None),
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

    fn on_exit(&self, callback: Box<dyn Fn(Result<i32, String>) + Send + Sync>) {
        *self.exit_callback.lock().unwrap() = Some(callback);
    }

    fn run(&self) {
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
                    *self.exit_code.lock().unwrap() = code;
                }
                ThreadResult::Failed(err) => {
                    if let Some(cb) = self.exit_callback.lock().unwrap().take() {
                        cb(Err(err));
                    }
                    return;
                }
            }
        }
        let code = *self.exit_code.lock().unwrap();
        if let Some(cb) = self.exit_callback.lock().unwrap().take() {
            cb(Ok(code));
        }
    }

    fn step(&self) -> Result<ExecutorStepResult, String> {
        let task = {
            let mut q = self.queue.lock().unwrap();
            q.pop_front()
        };

        let Some(task) = task else {
            return Ok(ExecutorStepResult::Blocked { timeout: None });
        };

        match task.run(100) {
            ThreadResult::Yielded(t) => {
                self.queue.lock().unwrap().push_back(t);
                Ok(ExecutorStepResult::Running)
            }
            ThreadResult::Blocked { timeout } => {
                let is_empty = self.queue.lock().unwrap().is_empty();
                if is_empty {
                    Ok(ExecutorStepResult::Blocked { timeout })
                } else {
                    Ok(ExecutorStepResult::Running)
                }
            }
            ThreadResult::Completed(code) => {
                *self.exit_code.lock().unwrap() = code;
                let is_empty = self.queue.lock().unwrap().is_empty();
                if is_empty {
                    Ok(ExecutorStepResult::Completed(code))
                } else {
                    Ok(ExecutorStepResult::Running)
                }
            }
            ThreadResult::Failed(err) => Err(err),
        }
    }
}

impl Default for SingleThreadExecutor {
    fn default() -> Self {
        Self::new()
    }
}
