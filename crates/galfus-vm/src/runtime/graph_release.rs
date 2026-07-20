use super::*;

impl<'a> VirtualMachine<'a> {
    #[allow(clippy::collapsible_if)]
    pub fn release_unreachable(&mut self) {
        use std::collections::{HashSet, VecDeque};

        self.allocations_since_release = 0;

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

        while let Some(obj_ref) = roots.pop_front() {
            if let Some(Some(obj)) = self.heap.get(obj_ref.raw()) {
                match obj {
                    HeapObject::Struct {
                        module_id,
                        layout_idx,
                        fields,
                    } => {
                        if let Some(layout) = self
                            .graph
                            .get(*module_id)
                            .unwrap()
                            .module
                            .struct_layouts
                            .get(layout_idx.raw() as usize)
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

        for idx in 0..self.heap.len() {
            if let Some(Some(HeapObject::Struct {
                module_id,
                layout_idx,
                fields,
            })) = self.heap.get_mut(idx)
            {
                let layout_idx_val = *layout_idx;
                if let Some(layout) = self
                    .graph
                    .get(*module_id)
                    .unwrap()
                    .module
                    .struct_layouts
                    .get(layout_idx_val.raw() as usize)
                {
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
}
