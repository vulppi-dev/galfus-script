use super::*;

impl VirtualMachine {
    pub(super) fn execute_system_instruction(
        &mut self,
        instr: Instruction,
    ) -> Result<ExecutionStep, VmError> {
        match instr {
            // Category E: Memory Ownership
            Instruction::Drop { reg } => {
                self.write_reg(reg, Value::Null)?;
            }

            // Category F: Transactional Shared Memory (stubbed/unimplemented)
            Instruction::TxStart { .. } => {
                let frame = self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
                frame.in_transaction = true;
            }
            Instruction::TxLoad { dest, obj, field } => {
                // Fall back to standard field load
                let obj_val = self.read_reg(obj)?;
                if let Value::Object(obj_ref) = obj_val {
                    let heap_obj = self.get_object(obj_ref)?;
                    if let HeapObject::Struct { fields, .. } = heap_obj {
                        let val = fields
                            .get(field.raw() as usize)
                            .cloned()
                            .ok_or(VmError::FieldOutOfBounds { index: field })?;
                        self.write_reg(dest, val)?;
                    } else {
                        return Err(VmError::TypeMismatch {
                            expected: "Struct object".to_string(),
                            found: format!("{:?}", heap_obj),
                        });
                    }
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Object reference".to_string(),
                        found: format!("{:?}", obj_val),
                    });
                }
            }
            Instruction::TxStore { obj, field, val } => {
                // Fall back to standard field store
                let obj_val = self.read_reg(obj)?;
                let val_to_store = self.read_reg(val)?;
                if let Value::Object(obj_ref) = obj_val {
                    let heap_obj = self.get_object_mut(obj_ref)?;
                    if let HeapObject::Struct { fields, .. } = heap_obj {
                        if (field.raw() as usize) < fields.len() {
                            fields[field.raw() as usize] = val_to_store;
                        } else {
                            return Err(VmError::FieldOutOfBounds { index: field });
                        }
                    } else {
                        return Err(VmError::TypeMismatch {
                            expected: "Struct object".to_string(),
                            found: format!("{:?}", heap_obj),
                        });
                    }
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Object reference".to_string(),
                        found: format!("{:?}", obj_val),
                    });
                }
            }
            Instruction::TxCommit { dest_reg } => {
                let frame = self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
                frame.in_transaction = false;
                self.write_reg(dest_reg, Value::Bool(true))?;
            }
            Instruction::TxRollback => {
                let frame = self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
                frame.in_transaction = false;
            }
            Instruction::Write { src } => {
                let val = self.read_reg(src)?;
                self.execute_write(val)?;
            }
            Instruction::Read { dest } => {
                let value = self.execute_read()?;
                self.write_reg(dest, value)?;
            }
            Instruction::Len { dest, src } => {
                let val = self.read_reg(src)?;
                if let Value::Object(obj_ref) = val {
                    let heap_obj = self.get_object(obj_ref)?;
                    match heap_obj {
                        HeapObject::Array { elements, .. } => {
                            self.write_reg(dest, Value::Int32(elements.len() as i32))?;
                        }
                        HeapObject::Tuple { elements, .. } => {
                            self.write_reg(dest, Value::Int32(elements.len() as i32))?;
                        }
                        _ => {
                            return Err(VmError::TypeMismatch {
                                expected: "Array or Tuple object".to_string(),
                                found: format!("{:?}", heap_obj),
                            });
                        }
                    }
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Object reference".to_string(),
                        found: format!("{:?}", val),
                    });
                }
            }
            Instruction::CopyArray {
                dest,
                dest_start,
                src,
            } => {
                let dest_val = self.read_reg(dest)?;
                let start_val = self.read_reg(dest_start)?;
                let src_val = self.read_reg(src)?;

                let start_idx = match start_val {
                    Value::Int8(x) if x >= 0 => x as usize,
                    Value::Int16(x) if x >= 0 => x as usize,
                    Value::Int32(x) if x >= 0 => x as usize,
                    Value::Int64(x) if x >= 0 => x as usize,
                    Value::Uint8(x) => x as usize,
                    Value::Uint16(x) => x as usize,
                    Value::Uint32(x) => x as usize,
                    Value::Uint64(x) => x as usize,
                    x => {
                        return Err(VmError::TypeMismatch {
                            expected: "positive index".to_string(),
                            found: format!("{:?}", x),
                        });
                    }
                };

                if let (Value::Object(dest_ref), Value::Object(src_ref)) =
                    (dest_val.clone(), src_val.clone())
                {
                    let src_elements = match self.get_object(src_ref)? {
                        HeapObject::Array { elements, .. } => elements.clone(),
                        HeapObject::Tuple { elements, .. } => elements.clone(),
                        other => {
                            return Err(VmError::TypeMismatch {
                                expected: "Array or Tuple".to_string(),
                                found: format!("{:?}", other),
                            });
                        }
                    };

                    let dest_obj = self.get_object_mut(dest_ref)?;
                    if let HeapObject::Array { elements, .. } = dest_obj {
                        if start_idx + src_elements.len() > elements.len() {
                            return Err(VmError::IndexOutOfBounds {
                                index: start_idx + src_elements.len() - 1,
                                len: elements.len(),
                            });
                        }
                        for (i, elem) in src_elements.into_iter().enumerate() {
                            elements[start_idx + i] = elem;
                        }
                    } else {
                        return Err(VmError::TypeMismatch {
                            expected: "Array object".to_string(),
                            found: format!("{:?}", dest_obj),
                        });
                    }
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Object references for source and destination".to_string(),
                        found: format!("{:?}, {:?}", dest_val, src_val),
                    });
                }
            }
            _ => unreachable!("instruction routed to the wrong runtime handler"),
        }

        Ok(ExecutionStep::Continue)
    }
}
