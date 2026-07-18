use super::*;

impl VirtualMachine {
    pub(super) fn execute_control_instruction(
        &mut self,
        instr: Instruction,
    ) -> Result<ExecutionStep, VmError> {
        match instr {
            // Category C: Control Flow & Subroutines
            Instruction::Jump { offset } => {
                let frame = self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
                let new_pc = (frame.pc as i32 + offset) as usize;
                frame.pc = new_pc;
            }
            Instruction::JumpTrue { cond, offset } => {
                let cond_val = self.read_reg(cond)?;
                match cond_val {
                    Value::Bool(b) => {
                        if b {
                            let frame =
                                self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
                            let new_pc = (frame.pc as i32 + offset) as usize;
                            frame.pc = new_pc;
                        }
                    }
                    x => {
                        return Err(VmError::TypeMismatch {
                            expected: "boolean conditional".to_string(),
                            found: format!("{:?}", x),
                        });
                    }
                }
            }
            Instruction::JumpFalse { cond, offset } => {
                let cond_val = self.read_reg(cond)?;
                match cond_val {
                    Value::Bool(b) => {
                        if !b {
                            let frame =
                                self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
                            let new_pc = (frame.pc as i32 + offset) as usize;
                            frame.pc = new_pc;
                        }
                    }
                    x => {
                        return Err(VmError::TypeMismatch {
                            expected: "boolean conditional".to_string(),
                            found: format!("{:?}", x),
                        });
                    }
                }
            }
            Instruction::JumpNull { val, offset } => {
                let val_read = self.read_reg(val)?;
                if matches!(val_read, Value::Null) {
                    let frame = self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
                    let new_pc = (frame.pc as i32 + offset) as usize;
                    frame.pc = new_pc;
                }
            }
            Instruction::Call {
                dest,
                func: func_idx,
                args_start,
                arg_count,
            } => {
                let callee = self
                    .image
                    .functions
                    .get(func_idx.raw() as usize)
                    .ok_or(VmError::FunctionOutOfBounds { index: func_idx })?;
                if arg_count != callee.param_count {
                    return Err(VmError::TypeMismatch {
                        expected: format!("{} arguments", callee.param_count),
                        found: format!("{} arguments", arg_count),
                    });
                }

                let mut callee_regs = vec![
                    Value::Null;
                    callee.param_count as usize
                        + callee.local_count as usize
                        + callee.temp_count as usize
                ];

                for (i, dest) in callee_regs.iter_mut().enumerate().take(arg_count as usize) {
                    let src_reg = Reg(args_start.raw() + i as u16);
                    let val = self.read_reg(src_reg)?;
                    *dest = val;
                }

                // Save destination register inside call frame to write return value back
                self.call_stack.push(CallFrame {
                    func_idx,
                    pc: 0,
                    registers: callee_regs,
                    return_dest: Some(dest),
                    in_transaction: false,
                });
            }
            Instruction::CallMethod {
                dest,
                obj,
                name_const,
                args_start,
                arg_count,
            } => {
                // Resolve method name from constant pool.
                let method_name = match self
                    .image
                    .constants
                    .constants
                    .get(name_const.raw() as usize)
                    .ok_or(VmError::ConstantOutOfBounds { index: name_const })?
                {
                    Constant::String(s) => s.clone(),
                    other => {
                        return Err(VmError::TypeMismatch {
                            expected: "string constant for method name".to_string(),
                            found: format!("{:?}", other),
                        });
                    }
                };

                if let Some(value) =
                    self.execute_array_iterator_method(obj, method_name.as_str())?
                {
                    self.write_reg(dest, value)?;
                    return Ok(ExecutionStep::Continue);
                }

                if method_name == "compare" {
                    let obj_val = self.read_reg(obj)?;
                    if !matches!(obj_val, Value::Object(_)) {
                        let arg_val = self.read_reg(Reg(args_start.raw() + 1))?;
                        let is_equal = obj_val == arg_val;
                        self.write_reg(dest, Value::Bool(is_equal))?;
                        return Ok(ExecutionStep::Continue);
                    }
                }

                let receiver_layout_name = match self.read_reg(obj)? {
                    Value::Object(obj_ref) => match self.get_object(obj_ref)? {
                        HeapObject::Struct { layout_idx, .. } => self
                            .image
                            .struct_layouts
                            .get(layout_idx.raw() as usize)
                            .map(|layout| layout.name.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                let qualified_name = receiver_layout_name
                    .as_ref()
                    .map(|layout_name| format!("{layout_name}::{method_name}"));

                let func_idx = if let Some(qualified_name) = qualified_name {
                    self.image
                        .functions
                        .iter()
                        .position(|function| {
                            let clean_name = if let Some(start) = function.name.find('<') {
                                if let Some(end) = function.name.find(">::") {
                                    format!(
                                        "{}::{}",
                                        &function.name[..start],
                                        &function.name[end + 3..]
                                    )
                                } else {
                                    function.name.clone()
                                }
                            } else {
                                function.name.clone()
                            };

                            clean_name == qualified_name
                                || clean_name.starts_with(&format!("{qualified_name}#"))
                        })
                        .map(|index| FuncIdx(index as u16))
                        .ok_or_else(|| {
                            let available = self
                                .image
                                .functions
                                .iter()
                                .map(|f| f.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ");
                            VmError::TypeMismatch {
                                expected: format!(
                                    "function named '{qualified_name}'. Available: {available}"
                                ),
                                found: "no matching function in image".to_string(),
                            }
                        })?
                } else {
                    self.image
                        .functions
                        .iter()
                        .position(|function| {
                            function.name == method_name
                                || function.name.ends_with(&format!("::{method_name}"))
                        })
                        .map(|index| FuncIdx(index as u16))
                        .ok_or_else(|| {
                            let available = self
                                .image
                                .functions
                                .iter()
                                .map(|f| f.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ");
                            VmError::TypeMismatch {
                                expected: format!(
                                    "function named '{method_name}'. Available: {available}"
                                ),
                                found: "no matching function in image".to_string(),
                            }
                        })?
                };

                let callee = &self.image.functions[func_idx.raw() as usize];
                if arg_count != callee.param_count {
                    return Err(VmError::TypeMismatch {
                        expected: format!("{} arguments", callee.param_count),
                        found: format!("{} arguments", arg_count),
                    });
                }

                let mut callee_regs = vec![
                    Value::Null;
                    callee.param_count as usize
                        + callee.local_count as usize
                        + callee.temp_count as usize
                ];

                for (i, dest) in callee_regs.iter_mut().enumerate().take(arg_count as usize) {
                    // First arg is obj, then args_start + 1, +2, ...
                    let src_reg = if i == 0 {
                        obj
                    } else {
                        Reg(args_start.raw() + i as u16)
                    };
                    *dest = self.read_reg(src_reg)?;
                }

                self.call_stack.push(CallFrame {
                    func_idx,
                    pc: 0,
                    registers: callee_regs,
                    return_dest: Some(dest),
                    in_transaction: false,
                });
            }
            Instruction::CallDynamic {
                dest,
                func_reg,
                args_start,
                arg_count,
            } => {
                let func_idx = match self.read_reg(func_reg)? {
                    Value::Function(func_idx) => func_idx,
                    value => {
                        return Err(VmError::TypeMismatch {
                            expected: "function value".to_string(),
                            found: format!("{:?}", value),
                        });
                    }
                };
                let callee = self
                    .image
                    .functions
                    .get(func_idx.raw() as usize)
                    .ok_or(VmError::FunctionOutOfBounds { index: func_idx })?;
                if arg_count != callee.param_count {
                    return Err(VmError::TypeMismatch {
                        expected: format!("{} arguments", callee.param_count),
                        found: format!("{} arguments", arg_count),
                    });
                }

                let mut callee_regs = vec![
                    Value::Null;
                    callee.param_count as usize
                        + callee.local_count as usize
                        + callee.temp_count as usize
                ];
                for (index, callee_reg) in
                    callee_regs.iter_mut().enumerate().take(arg_count as usize)
                {
                    let source = Reg(args_start.raw() + index as u16);
                    *callee_reg = self.read_reg(source)?;
                }

                self.call_stack.push(CallFrame {
                    func_idx,
                    pc: 0,
                    registers: callee_regs,
                    return_dest: Some(dest),
                    in_transaction: false,
                });
            }

            Instruction::Ret { src } => {
                let val = self.read_reg(src)?;
                let completed_frame = self.call_stack.pop().ok_or(VmError::EmptyCallStack)?;

                match completed_frame.return_dest {
                    Some(dest) => {
                        self.write_reg(dest, val)?;
                    }
                    None => {
                        return Ok(ExecutionStep::Return(val));
                    }
                }
            }
            Instruction::RetNull => {
                let completed_frame = self.call_stack.pop().ok_or(VmError::EmptyCallStack)?;

                match completed_frame.return_dest {
                    Some(dest) => {
                        self.write_reg(dest, Value::Null)?;
                    }
                    None => {
                        return Ok(ExecutionStep::Return(Value::Null));
                    }
                }
            }
            Instruction::Panic { const_idx } => {
                let constant = self
                    .image
                    .constants
                    .constants
                    .get(const_idx.raw() as usize)
                    .ok_or(VmError::ConstantOutOfBounds { index: const_idx })?;
                let msg = match constant {
                    Constant::String(s) => s.clone(),
                    Constant::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
                    x => format!("{:?}", x),
                };
                return Err(VmError::Panic { message: msg });
            }

            _ => unreachable!("instruction routed to the wrong runtime handler"),
        }

        Ok(ExecutionStep::Continue)
    }

    fn execute_array_iterator_method(
        &mut self,
        obj: Reg,
        method_name: &str,
    ) -> Result<Option<Value>, VmError> {
        let Value::Object(iterator_ref) = self.read_reg(obj)? else {
            return Ok(None);
        };

        let (array_ref, current) = match self.get_object(iterator_ref)? {
            HeapObject::Struct { layout_idx, fields } => {
                let Some(layout) = self.image.struct_layouts.get(layout_idx.raw() as usize) else {
                    return Ok(None);
                };
                if layout.name != "ArrayIterator" {
                    return Ok(None);
                }
                let array_ref = match fields.first() {
                    Some(Value::Object(array_ref)) => *array_ref,
                    value => {
                        return Err(VmError::TypeMismatch {
                            expected: "ArrayIterator array field".to_string(),
                            found: format!("{value:?}"),
                        });
                    }
                };
                let current = match fields.get(1) {
                    Some(Value::Int32(current)) => *current,
                    value => {
                        return Err(VmError::TypeMismatch {
                            expected: "ArrayIterator index field".to_string(),
                            found: format!("{value:?}"),
                        });
                    }
                };
                (array_ref, current)
            }
            _ => return Ok(None),
        };

        match method_name {
            "iter" => {
                let HeapObject::Struct { fields, .. } = self.get_object_mut(iterator_ref)? else {
                    unreachable!("iterator object was validated as a struct")
                };
                fields[1] = Value::Int32(0);
                Ok(Some(Value::Object(iterator_ref)))
            }
            "next" => {
                let value = match self.get_object(array_ref)? {
                    HeapObject::Array { elements, .. } => elements.get(current as usize).cloned(),
                    value => {
                        return Err(VmError::TypeMismatch {
                            expected: "ArrayIterator backing array".to_string(),
                            found: format!("{value:?}"),
                        });
                    }
                };
                if value.is_some() {
                    let HeapObject::Struct { fields, .. } = self.get_object_mut(iterator_ref)?
                    else {
                        unreachable!("iterator object was validated as a struct")
                    };
                    fields[1] = Value::Int32(current + 1);
                }
                Ok(Some(value.unwrap_or(Value::Null)))
            }
            _ => Ok(None),
        }
    }
}
