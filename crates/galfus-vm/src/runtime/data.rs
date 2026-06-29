use super::*;

impl VirtualMachine {
    pub(super) fn execute_data_instruction(
        &mut self,
        instr: Instruction,
    ) -> Result<ExecutionStep, VmError> {
        match instr {
            // Category A: Data Movement & Constants
            Instruction::LoadConst { dest, const_idx } => {
                let constant = self
                    .image
                    .constants
                    .constants
                    .get(const_idx.raw() as usize)
                    .ok_or(VmError::ConstantOutOfBounds { index: const_idx })?;
                let val = match constant {
                    Constant::Bool(b) => Value::Bool(*b),
                    Constant::Int32(i) => Value::Int32(*i),
                    Constant::Int64(i) => Value::Int64(*i),
                    Constant::Int(i) => Value::Int64(*i),
                    Constant::Float(f) => Value::Float64(*f),
                    Constant::String(s) => {
                        let obj = HeapObject::Array {
                            element_ty: TypeIdx(7), // Uint8
                            elements: s.bytes().map(Value::Uint8).collect(),
                        };
                        Value::Object(self.alloc(obj))
                    }
                    Constant::Bytes(b) => {
                        let obj = HeapObject::Array {
                            element_ty: TypeIdx(7), // Uint8
                            elements: b.iter().map(|&x| Value::Uint8(x)).collect(),
                        };
                        Value::Object(self.alloc(obj))
                    }
                };
                self.write_reg(dest, val)?;
            }
            Instruction::Move { dest, src } => {
                let val = self.read_reg(src)?;
                self.write_reg(dest, val)?;
            }
            Instruction::LoadGlobal { dest, global_idx } => {
                let val = self
                    .globals
                    .get(global_idx.raw() as usize)
                    .cloned()
                    .unwrap_or(Value::Null);
                self.write_reg(dest, val)?;
            }
            Instruction::StoreGlobal { global_idx, src } => {
                let val = self.read_reg(src)?;
                let idx = global_idx.raw() as usize;
                if idx >= self.globals.len() {
                    self.globals.resize(idx + 1, Value::Null);
                }
                self.globals[idx] = val;
            }
            Instruction::LoadNull { dest } => {
                self.write_reg(dest, Value::Null)?;
            }

            _ => unreachable!("instruction routed to the wrong runtime handler"),
        }

        Ok(ExecutionStep::Continue)
    }
}
