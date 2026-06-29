use super::*;

impl VirtualMachine {
    #[allow(clippy::collapsible_if)]
    pub fn release_unreachable(&mut self) {
        use std::collections::{HashSet, VecDeque};

        // 1. Gather all roots
        let mut roots = VecDeque::new();
        let mut reachable = HashSet::new();

        for val in &self.globals {
            if let Value::Object(obj_ref) = val {
                if reachable.insert(obj_ref.raw()) {
                    roots.push_back(*obj_ref);
                }
            }
        }

        for frame in &self.call_stack {
            for val in &frame.registers {
                if let Value::Object(obj_ref) = val {
                    if reachable.insert(obj_ref.raw()) {
                        roots.push_back(*obj_ref);
                    }
                }
            }
        }

        // 2. Traversal of strong references
        while let Some(obj_ref) = roots.pop_front() {
            if let Some(Some(obj)) = self.heap.get(obj_ref.raw()) {
                match obj {
                    HeapObject::Struct { layout_idx, fields } => {
                        if let Some(layout) =
                            self.image.struct_layouts.get(layout_idx.raw() as usize)
                        {
                            for (i, field_val) in fields.iter().enumerate() {
                                if let Value::Object(target_ref) = field_val {
                                    if let Some(field_layout) = layout.fields.get(i) {
                                        if field_layout.ownership != OwnershipKind::Weak {
                                            if reachable.insert(target_ref.raw()) {
                                                roots.push_back(*target_ref);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    HeapObject::Array { elements, .. } => {
                        for el in elements {
                            if let Value::Object(target_ref) = el {
                                if reachable.insert(target_ref.raw()) {
                                    roots.push_back(*target_ref);
                                }
                            }
                        }
                    }
                    HeapObject::Tuple { elements } => {
                        for el in elements {
                            if let Value::Object(target_ref) = el {
                                if reachable.insert(target_ref.raw()) {
                                    roots.push_back(*target_ref);
                                }
                            }
                        }
                    }
                    HeapObject::Choice { payload, .. } => {
                        if let Value::Object(target_ref) = payload {
                            if reachable.insert(target_ref.raw()) {
                                roots.push_back(*target_ref);
                            }
                        }
                    }
                }
            }
        }

        // 3. Find and release unreachable objects
        let mut dead_objects = Vec::new();
        for idx in 0..self.heap.len() {
            if self.heap[idx].is_some() && !reachable.contains(&idx) {
                dead_objects.push(idx);
            }
        }

        if dead_objects.is_empty() {
            return;
        }

        for &idx in &dead_objects {
            self.heap[idx] = None;
            self.free_slots.push(idx);
        }

        // 4. Invalidate weak observers pointing to dead objects
        for idx in 0..self.heap.len() {
            if let Some(Some(HeapObject::Struct { layout_idx, fields })) = self.heap.get_mut(idx) {
                let layout_idx_val = *layout_idx;
                if let Some(layout) = self.image.struct_layouts.get(layout_idx_val.raw() as usize) {
                    for (i, field_val) in fields.iter_mut().enumerate() {
                        if let Value::Object(target_ref) = field_val {
                            if dead_objects.contains(&target_ref.raw()) {
                                if let Some(field_layout) = layout.fields.get(i) {
                                    if field_layout.ownership == OwnershipKind::Weak {
                                        *field_val = Value::Null;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub(super) fn execute_write(&mut self, val: Value) -> Result<(), VmError> {
        let mut bytes = Vec::new();
        match val {
            Value::Object(obj_ref) => {
                let heap_obj = self.get_object(obj_ref)?;
                if let HeapObject::Array { elements, .. } = heap_obj {
                    for elem in elements {
                        match elem {
                            Value::Int8(b) => bytes.push(*b as u8),
                            Value::Uint8(b) => bytes.push(*b),
                            _ => {
                                let s = format!("{:?}", elem);
                                bytes.extend_from_slice(s.as_bytes());
                            }
                        }
                    }
                } else {
                    let s = format!("{:?}", heap_obj);
                    bytes.extend_from_slice(s.as_bytes());
                }
            }
            other => match other {
                Value::Null => bytes.extend_from_slice(b"null"),
                Value::Bool(b) => bytes.extend_from_slice(if b { b"true" } else { b"false" }),
                Value::Int8(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Int16(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Int32(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Int64(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Uint8(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Uint16(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Uint32(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Uint64(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Float32(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Float64(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Object(obj_ref) => {
                    let s = format!("ObjectRef({})", obj_ref.raw());
                    bytes.extend_from_slice(s.as_bytes());
                }
            },
        }
        let result = self
            .context
            .target
            .invoke(galfus_target::TargetCall::Write(bytes.as_slice()))
            .map_err(VmError::IoError)?;
        if !matches!(result, galfus_target::TargetResult::Success) {
            return Err(VmError::IoError(format!(
                "unexpected target result for write: {result:?}"
            )));
        }
        Ok(())
    }
}
