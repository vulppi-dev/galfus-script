use crate::queue::BlockedQueue;
use crate::registry::ThreadRegistry;
use galfus_contract::{RunnableTask, ThreadExecutor, ThreadResult};
use galfus_vm::thread::VirtualThread;
use galfus_vm::{ExecutionStep, VirtualMachine};
use std::sync::{Arc, Mutex};

pub struct RuntimeTask {
    pub thread: VirtualThread,
    pub vm: VirtualMachine,
    pub registry: Arc<Mutex<ThreadRegistry>>,
    pub blocked: Arc<Mutex<BlockedQueue>>,
    pub executor: Arc<dyn ThreadExecutor>,
}

impl RunnableTask for RuntimeTask {
    fn run(mut self: Box<Self>, budget: usize) -> ThreadResult {
        // execute_with_budget internally loops
        let step = match self.vm.execute_with_budget(&mut self.thread, budget) {
            Ok(step) => step,
            Err(e) => {
                return ThreadResult::Failed(e.to_string());
            }
        };

        match step {
            ExecutionStep::Continue => ThreadResult::Yielded(self),
            ExecutionStep::Return(val) => {
                let code = match val {
                    galfus_vm::VmValue::Int32(c) => c,
                    galfus_vm::VmValue::Null => 0,
                    _ => 0,
                };
                ThreadResult::Completed(code)
            }
            ExecutionStep::Blocked => ThreadResult::Blocked,
            ExecutionStep::SendMsg { target, msg } => {
                if target == 0 {
                    // Env/Host message. Currently we just log or drop?
                    // If we implement I/O Providers correctly, target 0 messages should go to Providers.
                    // For now we will just panic or ignore since it needs to be implemented.
                    // Actually in our previous implementation, we handled target 0 in `run_until_idle`.
                    println!("SendMsg to 0: {:?}", msg);
                    return ThreadResult::Yielded(self);
                }

                {
                    let mut registry_lock = self.registry.lock().unwrap();
                    if let Some(mailbox) =
                        registry_lock.get_mailbox(crate::registry::ThreadId(target))
                    {
                        // Deep copy here in real system. For now just clone (assuming immutable or primitive)
                        mailbox.lock().unwrap().push_back(msg);

                        let mut blocked_lock = self.blocked.lock().unwrap();
                        let target_id = crate::registry::ThreadId(target);
                        if blocked_lock.unblock(target_id)
                            && let Some(target_thread) = registry_lock.take(target_id)
                        {
                            let new_task = Box::new(RuntimeTask {
                                thread: target_thread,
                                vm: self.vm.clone(),
                                registry: self.registry.clone(),
                                blocked: self.blocked.clone(),
                                executor: self.executor.clone(),
                            });
                            self.executor.spawn(new_task);
                        }
                    }
                }
                ThreadResult::Yielded(self)
            }
        }
    }
}
