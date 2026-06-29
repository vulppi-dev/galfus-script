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

                for i in 0..arg_count as usize {
                    let src_reg = Reg(args_start.raw() + i as u16);
                    let val = self.read_reg(src_reg)?;
                    callee_regs[i] = val;
                }

                // Save destination register inside call frame to write return value back
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
                    // The callee PC was already advanced by 1 in Call,
                    // so we need to fetch the Call instruction to know where the destination register is!
                    let frame = self.call_stack.last().ok_or(VmError::EmptyCallStack)?;
                    let func = &self.image.functions[frame.func_idx.raw() as usize];
                    let call_pc = frame.pc - 1;
                    if let Some(Instruction::Call { dest, .. }) = func.instructions.get(call_pc) {
                        self.write_reg(*dest, val)?;
                    } else {
                        return Err(VmError::InvalidJumpTarget { pc });
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
                    if let Some(Instruction::Call { dest, .. }) = func.instructions.get(call_pc) {
                        self.write_reg(*dest, Value::Null)?;
                    } else {
                        return Err(VmError::InvalidJumpTarget { pc });
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
