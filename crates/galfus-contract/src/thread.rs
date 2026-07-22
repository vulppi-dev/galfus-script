/// Represents an encapsulated virtual thread, ready to run.
/// The host environment does not know its internals.
pub trait RunnableTask: Send {
    /// The host calls this method and provides a "budget" (e.g., number of instructions).
    /// The task runs until the budget is exhausted or it needs to pause.
    fn run(self: Box<Self>, budget: usize) -> ThreadResult;
}

/// The result returned after running a slice of a virtual thread.
pub enum ThreadResult {
    /// The thread consumed the budget but still has work to do.
    /// The host should re-queue it.
    Yielded(Box<dyn RunnableTask>),

    /// The thread finished execution successfully.
    Completed(i32),

    /// The thread encountered a critical error (panic).
    Failed(String),

    /// The thread needs to call a Provider or is waiting for a message.
    /// The Host should discard the task. The Runtime Orchestrator will
    /// wake it up and send it back to the Host when ready.
    Blocked {
        timeout: Option<std::time::Duration>,
    },
}

/// The Host must implement this trait to dictate how tasks are scheduled.
pub trait ThreadExecutor: Send + Sync {
    /// Allocates a unique, non-zero identity for a virtual thread.
    ///
    /// Implementations must never reuse an allocated value during one execution.
    fn allocate_thread_id(&self) -> u64;

    /// The Runtime calls this whenever a new thread is born or "woken up".
    fn spawn(&self, task: Box<dyn RunnableTask>);

    /// Runs the executor loop until no more tasks are active, returning the exit code or an error.
    fn run_until_idle(&self) -> Result<i32, String>;
}
