use crate::queue::BlockedQueue;
use crate::registry::ThreadRegistry;
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
            ExecutionStep::SendMsg { target, msg } => {
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
                                        let tid = self.thread_id.0 as usize;
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
            registry_lock.take(crate::registry::ThreadId(thread_id as u64))
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
            self.blocked
                .lock()
                .unwrap()
                .unblock(crate::registry::ThreadId(thread_id as u64));

            let new_task = Box::new(RuntimeTask {
                thread_id: crate::registry::ThreadId(thread_id as u64),
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
