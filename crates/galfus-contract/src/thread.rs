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

/// The result returned after running one step of the executor.
pub enum ExecutorStepResult {
    /// The executor still has tasks in the queue or is actively running them.
    Running,
    /// All tasks are blocked, waiting for external I/O or a timeout.
    Blocked {
        timeout: Option<std::time::Duration>,
    },
    /// All tasks have completed successfully. Contains the exit code of the entry thread.
    Completed(i32),
}

/// The Host must implement this trait to dictate how tasks are scheduled.
pub trait ThreadExecutor: Send + Sync {
    /// Allocates a unique, non-zero identity for a virtual thread.
    ///
    /// Implementations must never reuse an allocated value during one execution.
    fn allocate_thread_id(&self) -> u64;

    /// The Runtime calls this whenever a new thread is born or "woken up".
    fn spawn(&self, task: Box<dyn RunnableTask>);

    /// Sets the callback to be invoked when the executor completes its execution.
    fn on_exit(&self, callback: Box<dyn Fn(Result<i32, String>) + Send + Sync>);

    /// Runs the executor loop. Behavior (blocking vs non-blocking) depends on the implementation.
    fn run(&self);

    /// Executes a single task step from the queue, returning the current status.
    fn step(&self) -> Result<ExecutorStepResult, String> {
        unimplemented!("step is not implemented by default")
    }
}
