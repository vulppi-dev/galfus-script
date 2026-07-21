use crate::queue::BlockedQueue;
use crate::registry::{ThreadId, ThreadRegistry};
use galfus_contract::{RunnableTask, ThreadExecutor, ThreadResult};
use galfus_vm::thread::VirtualThread;
use galfus_vm::{ExecutionStep, VirtualMachine};
use std::sync::{Arc, Mutex};

pub struct RuntimeTask {
    pub thread_id: crate::registry::ThreadId,
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
            ExecutionStep::ReceiveFilter {
                dest,
                sender_id,
                timeout,
            } => {
                // If it reached here, control.rs has already checked the mailbox and found nothing.
                // We should add this thread to blocked queue.
                // If timeout is Some, we must set a timeout.
                if let Some(ms) = timeout {
                    self.blocked
                        .lock()
                        .unwrap()
                        .block_with_timeout(self.thread_id, ms);
                } else {
                    self.blocked.lock().unwrap().block(self.thread_id);
                }

                // We must put the thread back into the registry so others can send messages to it.
                self.registry
                    .lock()
                    .unwrap()
                    .register_with_id(self.thread_id, self.thread);
                ThreadResult::Blocked
            }
            ExecutionStep::CreateThread { dest, func, key } => {
                let mut new_thread = VirtualThread::new();

                // Store the string key if available
                if let galfus_vm::VmValue::Object(key_ref) = key {
                    if let Ok(galfus_vm::HeapObject::Array { elements, .. }) =
                        self.thread.heap.get_object(key_ref)
                    {
                        let mut string_key = String::new();
                        let mut is_string = true;
                        for e in elements {
                            if let galfus_vm::VmValue::Uint8(b) = e {
                                string_key.push(*b as char);
                            } else {
                                is_string = false;
                                break;
                            }
                        }
                        if is_string && !string_key.is_empty() {
                            new_thread.key = Some(string_key);
                        }
                    }
                }

                // Call prepare_function immediately without args so it is ready to execute, but don't spawn it yet
                if let galfus_vm::VmValue::Function { .. } = func {
                    new_thread.entry_func = Some(func);
                }

                // We register the thread, but we don't put it in the BlockedQueue or executor yet.
                let new_id = ThreadId::from_executor(self.executor.allocate_thread_id())
                    .expect("thread executor returned the reserved thread ID 0");
                self.registry.lock().unwrap().register(new_id, new_thread);
                let _ = self
                    .thread
                    .write_reg(dest, galfus_vm::VmValue::Int64(new_id.raw() as i64));

                ThreadResult::Yielded(self)
            }
            ExecutionStep::StartThread {
                dest,
                thread_id,
                arg,
            } => {
                let mut success = false;

                // Deep copy the argument to the new thread's heap
                let Some(target_id) = ThreadId::from_raw(thread_id) else {
                    let _ = self.thread.write_reg(dest, galfus_vm::VmValue::Bool(false));
                    return ThreadResult::Yielded(self);
                };

                if let Some(mut target_thread) = self.registry.lock().unwrap().take(target_id) {
                    let copied_arg = galfus_vm::thread::deep_copy_value(
                        &self.thread.heap,
                        &mut target_thread.heap,
                        &arg,
                    )
                    .unwrap_or(galfus_vm::VmValue::Null);

                    if let Some(galfus_vm::VmValue::Function {
                        module_id,
                        func_idx,
                    }) = target_thread.entry_func.clone()
                    {
                        let _ = self.vm.prepare_function(
                            &mut target_thread,
                            module_id,
                            func_idx,
                            vec![copied_arg],
                        );

                        self.registry
                            .lock()
                            .unwrap()
                            .register_with_id(target_id, target_thread);

                        if let Some(spawned_thread) = self.registry.lock().unwrap().take(target_id)
                        {
                            let new_task = Box::new(RuntimeTask {
                                thread_id: target_id,
                                thread: spawned_thread,
                                vm: self.vm.clone(),
                                registry: self.registry.clone(),
                                blocked: self.blocked.clone(),
                                executor: self.executor.clone(),
                            });
                            self.executor.spawn(new_task);
                            success = true;
                        }
                    } else {
                        // Put it back if no function
                        self.registry
                            .lock()
                            .unwrap()
                            .register_with_id(target_id, target_thread);
                    }
                }

                let _ = self
                    .thread
                    .write_reg(dest, galfus_vm::VmValue::Bool(success));
                ThreadResult::Yielded(self)
            }
            ExecutionStep::SendMsg { dest, target, msg } => {
                if target == 0 {
                    let host_val = to_host_value(&self.thread.heap, msg);
                    if let Some(HostValue::Array(mut arr)) = host_val {
                        if !arr.is_empty() {
                            let method_opt = match arr.remove(0) {
                                HostValue::String(s) => Some(s),
                                HostValue::Bytes(b) => String::from_utf8(b).ok(),
                                _ => None,
                            };
                            if let Some(method) = method_opt {
                                let p_opt = self.vm.shared_providers();
                                if let Some(providers) = &p_opt {
                                    let mut p_lock = providers.lock().unwrap();
                                    if let Some(host) = p_lock.host_mut() {
                                        let injector = Arc::new(RuntimeInjector {
                                            registry: self.registry.clone(),
                                            blocked: self.blocked.clone(),
                                            executor: self.executor.clone(),
                                            vm: self.vm.clone(),
                                        });
                                        let tid = self.thread_id.raw() as usize;
                                        self.registry
                                            .lock()
                                            .unwrap()
                                            .register_with_id(self.thread_id, self.thread);
                                        self.blocked.lock().unwrap().block(self.thread_id);
                                        host.dispatch(tid, &method, &arr, injector);
                                        return ThreadResult::Blocked;
                                    }
                                }
                            }
                        }
                    }
                    return ThreadResult::Failed(
                        "Invalid SendMsg payload to Host or HostProvider missing".to_string(),
                    );
                }

                let Some(target_id) = ThreadId::from_raw(target) else {
                    let _ = self.thread.write_reg(dest, galfus_vm::VmValue::Int32(1));
                    return ThreadResult::Yielded(self);
                };

                {
                    let mut registry_lock = self.registry.lock().unwrap();

                    let mut success = false;

                    // The target thread might be currently running (so not in registry),
                    // or in the registry.
                    if let Some(mut target_thread) = registry_lock.take(target_id) {
                        if let Ok(copied_msg) = galfus_vm::thread::deep_copy_value(
                            &self.thread.heap,
                            &mut target_thread.heap,
                            &msg,
                        ) {
                            target_thread
                                .mailbox
                                .lock()
                                .unwrap()
                                .push_back((self.thread_id.raw(), copied_msg));
                            success = true;
                        }

                        registry_lock.register_with_id(target_id, target_thread);

                        if success {
                            let mut blocked_lock = self.blocked.lock().unwrap();
                            if blocked_lock.unblock(target_id)
                                && let Some(target_thread) = registry_lock.take(target_id)
                            {
                                let new_task = Box::new(RuntimeTask {
                                    thread_id: target_id,
                                    thread: target_thread,
                                    vm: self.vm.clone(),
                                    registry: self.registry.clone(),
                                    blocked: self.blocked.clone(),
                                    executor: self.executor.clone(),
                                });
                                self.executor.spawn(new_task);
                            }
                        }
                    } else {
                        // Thread is likely running in executor and not in registry, so we can't deep copy!
                        // For now, if we can't deep copy, we fail the send (return 1).
                        // In a more robust system, we would queue the serialized message and deserialize later.
                        success = false;
                    }

                    let _ = self
                        .thread
                        .write_reg(dest, galfus_vm::VmValue::Int32(if success { 0 } else { 1 }));
                }
                ThreadResult::Yielded(self)
            }
        }
    }
}

use galfus_contract::HostValue;
use galfus_vm::{HeapObject, VmValue, thread::PrivateHeap};

fn to_host_value(heap: &PrivateHeap, val: VmValue) -> Option<HostValue> {
    match val {
        VmValue::Null => Some(HostValue::Null),
        VmValue::Int32(v) => Some(HostValue::Int32(v)),
        VmValue::Object(r) => {
            let obj = heap.get_object(r).ok()?;
            match obj {
                HeapObject::Array {
                    element_ty: _,
                    elements,
                } => {
                    // Could be bytes or array
                    // Check if it looks like bytes (all elements are Uint8)
                    // For now, let us just check if it is all uint8
                    let mut is_bytes = true;
                    let mut bytes = Vec::new();
                    for e in elements {
                        if let VmValue::Uint8(b) = e {
                            bytes.push(*b);
                        } else {
                            is_bytes = false;
                            break;
                        }
                    }
                    if is_bytes {
                        return Some(HostValue::Bytes(bytes));
                    }
                    // Otherwise recursive
                    let mut arr = Vec::new();
                    for e in elements {
                        arr.push(to_host_value(heap, e.clone())?);
                    }
                    Some(HostValue::Array(arr))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn from_host_value(heap: &mut PrivateHeap, val: HostValue, vm: &VirtualMachine) -> VmValue {
    match val {
        HostValue::Null => VmValue::Null,
        HostValue::Int32(v) => VmValue::Int32(v),
        HostValue::String(s) => {
            let elements = s.into_bytes().into_iter().map(VmValue::Uint8).collect();
            VmValue::Object(heap.alloc(HeapObject::Array {
                element_ty: galfus_bytecode::instruction::TypeIdx(0),
                elements,
            }))
        }
        HostValue::Bytes(b) => {
            let elements = b.into_iter().map(VmValue::Uint8).collect();
            // We need the type index for uint8
            // We can just use a dummy type index for now since we do not do strict checking on Host values
            VmValue::Object(heap.alloc(HeapObject::Array {
                element_ty: galfus_bytecode::instruction::TypeIdx(0),
                elements,
            }))
        }
        HostValue::Array(arr) => {
            let elements = arr
                .into_iter()
                .map(|e| from_host_value(heap, e, vm))
                .collect();
            VmValue::Object(heap.alloc(HeapObject::Array {
                element_ty: galfus_bytecode::instruction::TypeIdx(0),
                elements,
            }))
        }
    }
}

struct RuntimeInjector {
    registry: Arc<Mutex<ThreadRegistry>>,
    blocked: Arc<Mutex<BlockedQueue>>,
    executor: Arc<dyn ThreadExecutor>,
    vm: VirtualMachine,
}

impl galfus_contract::MessageInjector for RuntimeInjector {
    fn inject_system_response(&self, thread_id: usize, response: galfus_contract::HostResponse) {
        let mut registry_lock = self.registry.lock().unwrap();
        if let Some(mut target_thread) =
            ThreadId::from_raw(thread_id as u64).and_then(|thread_id| registry_lock.take(thread_id))
        {
            let val = match response {
                galfus_contract::HostResponse::Success(v) => {
                    from_host_value(&mut target_thread.heap, v, &self.vm)
                }
                galfus_contract::HostResponse::Error(e) => {
                    from_host_value(&mut target_thread.heap, HostValue::String(e), &self.vm)
                }
            };
            target_thread.system_response = Some(val);

            // Re-spawn the thread
            self.blocked.lock().unwrap().unblock(
                ThreadId::from_raw(thread_id as u64).expect("host response thread ID is non-zero"),
            );

            let new_task = Box::new(RuntimeTask {
                thread_id: ThreadId::from_raw(thread_id as u64)
                    .expect("host response thread ID is non-zero"),
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
