use crate::error::VmError;
use crate::runtime::Value;
use crate::runtime::{CallFrame, HeapObject, RuntimeModuleState, VmObjectRef};
use galfus_bytecode::instruction::Reg;
use galfus_core::ModuleId;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

#[cfg(test)]
mod tests;

pub struct PrivateHeap {
    pub objects: Vec<Option<HeapObject>>,
    pub free_slots: Vec<usize>,
    pub allocations_since_release: usize,
}

impl Default for PrivateHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivateHeap {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            free_slots: Vec::new(),
            allocations_since_release: 0,
        }
    }

    pub fn alloc(&mut self, obj: HeapObject) -> VmObjectRef {
        self.allocations_since_release += 1;

        if let Some(idx) = self.free_slots.pop() {
            self.objects[idx] = Some(obj);
            VmObjectRef(idx)
        } else {
            let idx = self.objects.len();
            self.objects.push(Some(obj));
            VmObjectRef(idx)
        }
    }

    pub fn get_object(&self, obj_ref: VmObjectRef) -> Result<&HeapObject, VmError> {
        let idx = obj_ref.raw();
        if idx < self.objects.len()
            && let Some(ref obj) = self.objects[idx]
        {
            return Ok(obj);
        }
        Err(VmError::InvalidObjectReference)
    }

    pub fn get_object_mut(&mut self, obj_ref: VmObjectRef) -> Result<&mut HeapObject, VmError> {
        let idx = obj_ref.raw();
        if idx < self.objects.len()
            && let Some(ref mut obj) = self.objects[idx]
        {
            return Ok(obj);
        }
        Err(VmError::InvalidObjectReference)
    }

    pub fn free_object(&mut self, obj_ref: VmObjectRef) -> Result<(), VmError> {
        let idx = obj_ref.raw();
        if idx < self.objects.len() && self.objects[idx].is_some() {
            self.objects[idx] = None;
            self.free_slots.push(idx);
            return Ok(());
        }
        Err(VmError::InvalidObjectReference)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Created,
    Running,
    Exited(i32),
}

pub struct MailboxMessage {
    pub sender_id: u64,
    pub data: Vec<u8>,
}

pub struct VirtualThread {
    pub call_stack: Vec<CallFrame>,
    pub system_response: Option<crate::VmValue>,
    pub heap: PrivateHeap,
    pub module_states: HashMap<ModuleId, RuntimeModuleState>,
    pub mailbox: Arc<Mutex<VecDeque<MailboxMessage>>>,
    pub state: ThreadState,
    pub key: Option<String>,
    pub entry_func: Option<crate::runtime::Value>,
}

impl Default for VirtualThread {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadState {
    pub fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }

    pub fn is_exited(self) -> bool {
        matches!(self, Self::Exited(_))
    }

    pub fn exit_reason(self) -> Option<i32> {
        match self {
            Self::Exited(code) => Some(code),
            Self::Created | Self::Running => None,
        }
    }
}

impl VirtualThread {
    pub fn new() -> Self {
        Self {
            call_stack: Vec::new(),
            system_response: None,
            heap: PrivateHeap::new(),
            module_states: HashMap::new(),
            mailbox: Arc::new(Mutex::new(VecDeque::new())),
            state: ThreadState::Created,
            key: None,
            entry_func: None,
        }
    }

    pub fn mark_running(&mut self) -> bool {
        if self.state != ThreadState::Created {
            return false;
        }

        self.state = ThreadState::Running;
        true
    }

    pub fn mark_exited(&mut self, code: i32) -> bool {
        if !self.state.is_running() {
            return false;
        }

        self.state = ThreadState::Exited(code);
        true
    }

    pub fn module_state(&self, module_id: ModuleId) -> Option<&RuntimeModuleState> {
        self.module_states.get(&module_id)
    }

    pub fn is_module_initialized(&self, module_id: ModuleId) -> bool {
        self.module_state(module_id)
            .is_some_and(|state| state.initialized)
    }

    pub fn mark_module_initialized(&mut self, module_id: ModuleId) {
        self.module_states.entry(module_id).or_default().initialized = true;
    }

    pub fn read_reg(&self, reg: Reg) -> Result<Value, VmError> {
        let frame = self.call_stack.last().ok_or(VmError::EmptyCallStack)?;
        frame
            .registers
            .get(reg.raw() as usize)
            .cloned()
            .ok_or(VmError::RegisterOutOfBounds { reg })
    }

    pub fn write_reg(&mut self, reg: Reg, val: Value) -> Result<(), VmError> {
        let frame = self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
        if (reg.raw() as usize) < frame.registers.len() {
            frame.registers[reg.raw() as usize] = val;
            Ok(())
        } else {
            Err(VmError::RegisterOutOfBounds { reg })
        }
    }
}

pub fn deep_copy_value(
    src_heap: &PrivateHeap,
    dst_heap: &mut PrivateHeap,
    val: &Value,
) -> Result<Value, VmError> {
    match val {
        Value::Null => Ok(Value::Null),
        Value::Bool(b) => Ok(Value::Bool(*b)),
        Value::Int8(i) => Ok(Value::Int8(*i)),
        Value::Int16(i) => Ok(Value::Int16(*i)),
        Value::Int32(i) => Ok(Value::Int32(*i)),
        Value::Int64(i) => Ok(Value::Int64(*i)),
        Value::Uint8(i) => Ok(Value::Uint8(*i)),
        Value::Uint16(i) => Ok(Value::Uint16(*i)),
        Value::Uint32(i) => Ok(Value::Uint32(*i)),
        Value::Uint64(i) => Ok(Value::Uint64(*i)),
        Value::Float32(f) => Ok(Value::Float32(*f)),
        Value::Float64(f) => Ok(Value::Float64(*f)),
        Value::Function {
            module_id,
            func_idx,
        } => Ok(Value::Function {
            module_id: *module_id,
            func_idx: *func_idx,
        }),
        Value::Object(obj_ref) => {
            let obj = src_heap.get_object(*obj_ref)?;
            let new_obj = match obj {
                HeapObject::Struct {
                    module_id,
                    layout_idx,
                    fields,
                } => {
                    let mut new_fields = Vec::with_capacity(fields.len());
                    for f in fields {
                        new_fields.push(deep_copy_value(src_heap, dst_heap, f)?);
                    }
                    HeapObject::Struct {
                        module_id: *module_id,
                        layout_idx: *layout_idx,
                        fields: new_fields,
                    }
                }
                HeapObject::Array {
                    element_ty,
                    elements,
                } => {
                    let mut new_elements = Vec::with_capacity(elements.len());
                    for e in elements {
                        new_elements.push(deep_copy_value(src_heap, dst_heap, e)?);
                    }
                    HeapObject::Array {
                        element_ty: *element_ty,
                        elements: new_elements,
                    }
                }
                HeapObject::Tuple { elements } => {
                    let mut new_elements = Vec::with_capacity(elements.len());
                    for e in elements {
                        new_elements.push(deep_copy_value(src_heap, dst_heap, e)?);
                    }
                    HeapObject::Tuple {
                        elements: new_elements,
                    }
                }
                HeapObject::Choice {
                    module_id,
                    layout_idx,
                    variant_idx,
                    payload,
                } => {
                    let new_payload = deep_copy_value(src_heap, dst_heap, payload)?;
                    HeapObject::Choice {
                        module_id: *module_id,
                        layout_idx: *layout_idx,
                        variant_idx: *variant_idx,
                        payload: new_payload,
                    }
                }
            };
            Ok(Value::Object(dst_heap.alloc(new_obj)))
        }
    }
}
