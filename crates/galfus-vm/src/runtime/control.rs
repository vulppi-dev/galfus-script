use super::*;

impl VirtualMachine {
    pub(super) fn execute_control_instruction(
        &mut self,
        instr: Instruction,
        pc: usize,
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
                dest: _,
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
                    in_transaction: false,
                });
            }
            Instruction::CallMethod {
                dest: _,
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

                // Look up a function whose name matches the method name exactly
                // or ends with `::<method_name>`.
                let func_idx = self
                    .image
                    .functions
                    .iter()
                    .position(|f| {
                        f.name == method_name || f.name.ends_with(&format!("::{method_name}"))
                    })
                    .map(|i| FuncIdx(i as u16))
                    .ok_or_else(|| VmError::TypeMismatch {
                        expected: format!("function named '{method_name}'"),
                        found: "no matching function in image".to_string(),
                    })?;

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
                    in_transaction: false,
                });
            }

            Instruction::Ret { src } => {
                let val = self.read_reg(src)?;
                self.call_stack.pop();
                if self.call_stack.is_empty() {
                    return Ok(ExecutionStep::Return(val));
                } else {
                    // The callee PC was already advanced by 1 in Call/CallMethod,
                    // so we fetch the call instruction to know the destination register.
                    let frame = self.call_stack.last().ok_or(VmError::EmptyCallStack)?;
                    let func = &self.image.functions[frame.func_idx.raw() as usize];
                    let call_pc = frame.pc - 1;
                    match func.instructions.get(call_pc) {
                        Some(Instruction::Call { dest, .. })
                        | Some(Instruction::CallMethod { dest, .. }) => {
                            self.write_reg(*dest, val)?;
                        }
                        _ => return Err(VmError::InvalidJumpTarget { pc }),
                    }
                }
            }
            Instruction::RetNull => {
                self.call_stack.pop();
                if self.call_stack.is_empty() {
                    return Ok(ExecutionStep::Return(Value::Null));
                } else {
                    let frame = self.call_stack.last().ok_or(VmError::EmptyCallStack)?;
                    let func = &self.image.functions[frame.func_idx.raw() as usize];
                    let call_pc = frame.pc - 1;
                    match func.instructions.get(call_pc) {
                        Some(Instruction::Call { dest, .. })
                        | Some(Instruction::CallMethod { dest, .. }) => {
                            self.write_reg(*dest, Value::Null)?;
                        }
                        _ => return Err(VmError::InvalidJumpTarget { pc }),
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
}
