use crate::error::{StackFrameInfo, VmError, VmPanic};
use galfus_core::image::instruction::{
    ChoiceLayoutIdx, FuncIdx, Instruction, Reg, StructLayoutIdx, TypeIdx,
};
use galfus_core::image::{Constant, ImageType, ModuleImage, ObjectRef, OwnershipKind, Value};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq)]
pub enum HeapObject {
    Struct {
        layout_idx: StructLayoutIdx,
        fields: Vec<Value>,
    },
    Array {
        element_ty: TypeIdx,
        elements: Vec<Value>,
    },
    Tuple {
        elements: Vec<Value>,
    },
    Choice {
        layout_idx: ChoiceLayoutIdx,
        variant_idx: u16,
        payload: Value,
    },
}

pub trait IoHandler: Send + Sync {
    fn write(&mut self, data: &[u8]) -> Result<(), VmError>;
    fn read(&mut self) -> Result<Option<u8>, VmError>;
}

pub struct DefaultIoHandler;

impl IoHandler for DefaultIoHandler {
    fn write(&mut self, data: &[u8]) -> Result<(), VmError> {
        use std::io::Write;
        std::io::stdout()
            .write_all(data)
            .map_err(|e| VmError::IoError(e.to_string()))?;
        std::io::stdout()
            .flush()
            .map_err(|e| VmError::IoError(e.to_string()))?;
        Ok(())
    }

    fn read(&mut self) -> Result<Option<u8>, VmError> {
        use std::io::Read;
        let mut buf = [0u8; 1];
        match std::io::stdin().read_exact(&mut buf) {
            Ok(_) => Ok(Some(buf[0])),
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(VmError::IoError(e.to_string())),
        }
    }
}

pub struct CallFrame {
    pub func_idx: FuncIdx,
    pub pc: usize,
    pub registers: Vec<Value>,
    pub in_transaction: bool,
}

pub struct VirtualMachine {
    pub image: ModuleImage,
    pub globals: Vec<Value>,
    pub heap: Vec<Option<HeapObject>>,
    pub free_slots: Vec<usize>,
    pub call_stack: Vec<CallFrame>,
    pub io_handler: Box<dyn IoHandler>,
}

macro_rules! impl_binary_op {
    ($self:expr, $dest:expr, $lhs:expr, $rhs:expr, +) => {{
        let lhs_val = $self.read_reg($lhs)?;
        let rhs_val = $self.read_reg($rhs)?;
        let res = match (lhs_val, rhs_val) {
            (Value::Int8(l), Value::Int8(r)) => Value::Int8(l.wrapping_add(r)),
            (Value::Int16(l), Value::Int16(r)) => Value::Int16(l.wrapping_add(r)),
            (Value::Int32(l), Value::Int32(r)) => Value::Int32(l.wrapping_add(r)),
            (Value::Int64(l), Value::Int64(r)) => Value::Int64(l.wrapping_add(r)),
            (Value::Uint8(l), Value::Uint8(r)) => Value::Uint8(l.wrapping_add(r)),
            (Value::Uint16(l), Value::Uint16(r)) => Value::Uint16(l.wrapping_add(r)),
            (Value::Uint32(l), Value::Uint32(r)) => Value::Uint32(l.wrapping_add(r)),
            (Value::Uint64(l), Value::Uint64(r)) => Value::Uint64(l.wrapping_add(r)),
            (Value::Float32(l), Value::Float32(r)) => Value::Float32(l + r),
            (Value::Float64(l), Value::Float64(r)) => Value::Float64(l + r),
            (l, r) => {
                return Err(VmError::TypeMismatch {
                    expected: "matching numeric types".to_string(),
                    found: format!("{:?} and {:?}", l, r),
                });
            }
        };
        $self.write_reg($dest, res)?;
    }};
    ($self:expr, $dest:expr, $lhs:expr, $rhs:expr, -) => {{
        let lhs_val = $self.read_reg($lhs)?;
        let rhs_val = $self.read_reg($rhs)?;
        let res = match (lhs_val, rhs_val) {
            (Value::Int8(l), Value::Int8(r)) => Value::Int8(l.wrapping_sub(r)),
            (Value::Int16(l), Value::Int16(r)) => Value::Int16(l.wrapping_sub(r)),
            (Value::Int32(l), Value::Int32(r)) => Value::Int32(l.wrapping_sub(r)),
            (Value::Int64(l), Value::Int64(r)) => Value::Int64(l.wrapping_sub(r)),
            (Value::Uint8(l), Value::Uint8(r)) => Value::Uint8(l.wrapping_sub(r)),
            (Value::Uint16(l), Value::Uint16(r)) => Value::Uint16(l.wrapping_sub(r)),
            (Value::Uint32(l), Value::Uint32(r)) => Value::Uint32(l.wrapping_sub(r)),
            (Value::Uint64(l), Value::Uint64(r)) => Value::Uint64(l.wrapping_sub(r)),
            (Value::Float32(l), Value::Float32(r)) => Value::Float32(l - r),
            (Value::Float64(l), Value::Float64(r)) => Value::Float64(l - r),
            (l, r) => {
                return Err(VmError::TypeMismatch {
                    expected: "matching numeric types".to_string(),
                    found: format!("{:?} and {:?}", l, r),
                });
            }
        };
        $self.write_reg($dest, res)?;
    }};
    ($self:expr, $dest:expr, $lhs:expr, $rhs:expr, *) => {{
        let lhs_val = $self.read_reg($lhs)?;
        let rhs_val = $self.read_reg($rhs)?;
        let res = match (lhs_val, rhs_val) {
            (Value::Int8(l), Value::Int8(r)) => Value::Int8(l.wrapping_mul(r)),
            (Value::Int16(l), Value::Int16(r)) => Value::Int16(l.wrapping_mul(r)),
            (Value::Int32(l), Value::Int32(r)) => Value::Int32(l.wrapping_mul(r)),
            (Value::Int64(l), Value::Int64(r)) => Value::Int64(l.wrapping_mul(r)),
            (Value::Uint8(l), Value::Uint8(r)) => Value::Uint8(l.wrapping_mul(r)),
            (Value::Uint16(l), Value::Uint16(r)) => Value::Uint16(l.wrapping_mul(r)),
            (Value::Uint32(l), Value::Uint32(r)) => Value::Uint32(l.wrapping_mul(r)),
            (Value::Uint64(l), Value::Uint64(r)) => Value::Uint64(l.wrapping_mul(r)),
            (Value::Float32(l), Value::Float32(r)) => Value::Float32(l * r),
            (Value::Float64(l), Value::Float64(r)) => Value::Float64(l * r),
            (l, r) => {
                return Err(VmError::TypeMismatch {
                    expected: "matching numeric types".to_string(),
                    found: format!("{:?} and {:?}", l, r),
                });
            }
        };
        $self.write_reg($dest, res)?;
    }};
}

macro_rules! impl_bitwise_op {
    ($self:expr, $dest:expr, $lhs:expr, $rhs:expr, $op:tt) => {{
        let lhs_val = $self.read_reg($lhs)?;
        let rhs_val = $self.read_reg($rhs)?;
        let res = match (lhs_val, rhs_val) {
            (Value::Int8(l), Value::Int8(r)) => Value::Int8(l $op r),
            (Value::Int16(l), Value::Int16(r)) => Value::Int16(l $op r),
            (Value::Int32(l), Value::Int32(r)) => Value::Int32(l $op r),
            (Value::Int64(l), Value::Int64(r)) => Value::Int64(l $op r),
            (Value::Uint8(l), Value::Uint8(r)) => Value::Uint8(l $op r),
            (Value::Uint16(l), Value::Uint16(r)) => Value::Uint16(l $op r),
            (Value::Uint32(l), Value::Uint32(r)) => Value::Uint32(l $op r),
            (Value::Uint64(l), Value::Uint64(r)) => Value::Uint64(l $op r),
            (l, r) => return Err(VmError::TypeMismatch {
                expected: "matching integer types".to_string(),
                found: format!("{:?} and {:?}", l, r),
            }),
        };
        $self.write_reg($dest, res)?;
    }};
}

impl VirtualMachine {
    pub fn new(image: ModuleImage) -> Self {
        Self {
            image,
            globals: Vec::new(),
            heap: Vec::new(),
            free_slots: Vec::new(),
            call_stack: Vec::new(),
            io_handler: Box::new(DefaultIoHandler),
        }
    }

    pub fn with_io_handler(mut self, io_handler: Box<dyn IoHandler>) -> Self {
        self.io_handler = io_handler;
        self
    }

    pub fn alloc(&mut self, obj: HeapObject) -> ObjectRef {
        if let Some(idx) = self.free_slots.pop() {
            self.heap[idx] = Some(obj);
            ObjectRef(idx)
        } else {
            let idx = self.heap.len();
            self.heap.push(Some(obj));
            ObjectRef(idx)
        }
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

    pub fn run_function(&mut self, func_idx: FuncIdx, args: Vec<Value>) -> Result<Value, VmPanic> {
        if (func_idx.raw() as usize) >= self.image.functions.len() {
            return Err(VmPanic {
                error: VmError::FunctionOutOfBounds { index: func_idx },
                stack_trace: vec![],
            });
        }

        let func = &self.image.functions[func_idx.raw() as usize];
        if args.len() != func.param_count as usize {
            return Err(VmPanic {
                error: VmError::TypeMismatch {
                    expected: format!("{} arguments", func.param_count),
                    found: format!("{} arguments", args.len()),
                },
                stack_trace: vec![],
            });
        }

        self.call_stack.clear();
        let total_regs =
            func.param_count as usize + func.local_count as usize + func.temp_count as usize;
        let mut registers = vec![Value::Null; total_regs];
        for (i, val) in args.into_iter().enumerate() {
            registers[i] = val;
        }

        self.call_stack.push(CallFrame {
            func_idx,
            pc: 0,
            registers,
            in_transaction: false,
        });

        match self.execute_loop() {
            Ok(val) => Ok(val),
            Err(err) => {
                let mut stack_trace = Vec::new();
                for frame in self.call_stack.iter().rev() {
                    let f_name = self
                        .image
                        .functions
                        .get(frame.func_idx.raw() as usize)
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| format!("func#{}", frame.func_idx.raw()));
                    stack_trace.push(StackFrameInfo {
                        function_name: f_name,
                        pc: frame.pc,
                    });
                }
                Err(VmPanic {
                    error: err,
                    stack_trace,
                })
            }
        }
    }

    fn execute_loop(&mut self) -> Result<Value, VmError> {
        loop {
            let (instr, pc) = {
                let frame = self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
                let func = &self.image.functions[frame.func_idx.raw() as usize];
                if frame.pc >= func.instructions.len() {
                    return Err(VmError::InstructionPointerOutOfBounds { pc: frame.pc });
                }
                let instr = func.instructions[frame.pc];
                let pc = frame.pc;
                frame.pc += 1;
                (instr, pc)
            };

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

                // Category B: Unary & Binary Operations
                Instruction::Add { dest, lhs, rhs } => {
                    impl_binary_op!(self, dest, lhs, rhs, +);
                }
                Instruction::Sub { dest, lhs, rhs } => {
                    impl_binary_op!(self, dest, lhs, rhs, -);
                }
                Instruction::Mul { dest, lhs, rhs } => {
                    impl_binary_op!(self, dest, lhs, rhs, *);
                }
                Instruction::Div { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    let res = match (lhs_val, rhs_val) {
                        (Value::Int8(l), Value::Int8(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Int8(l / r)
                        }
                        (Value::Int16(l), Value::Int16(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Int16(l / r)
                        }
                        (Value::Int32(l), Value::Int32(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Int32(l / r)
                        }
                        (Value::Int64(l), Value::Int64(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Int64(l / r)
                        }
                        (Value::Uint8(l), Value::Uint8(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Uint8(l / r)
                        }
                        (Value::Uint16(l), Value::Uint16(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Uint16(l / r)
                        }
                        (Value::Uint32(l), Value::Uint32(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Uint32(l / r)
                        }
                        (Value::Uint64(l), Value::Uint64(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Uint64(l / r)
                        }
                        (Value::Float32(l), Value::Float32(r)) => {
                            if r == 0.0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Float32(l / r)
                        }
                        (Value::Float64(l), Value::Float64(r)) => {
                            if r == 0.0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Float64(l / r)
                        }
                        (l, r) => {
                            return Err(VmError::TypeMismatch {
                                expected: "matching numeric types".to_string(),
                                found: format!("{:?} and {:?}", l, r),
                            });
                        }
                    };
                    self.write_reg(dest, res)?;
                }
                Instruction::Rem { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    let res = match (lhs_val, rhs_val) {
                        (Value::Int8(l), Value::Int8(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Int8(l % r)
                        }
                        (Value::Int16(l), Value::Int16(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Int16(l % r)
                        }
                        (Value::Int32(l), Value::Int32(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Int32(l % r)
                        }
                        (Value::Int64(l), Value::Int64(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Int64(l % r)
                        }
                        (Value::Uint8(l), Value::Uint8(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Uint8(l % r)
                        }
                        (Value::Uint16(l), Value::Uint16(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Uint16(l % r)
                        }
                        (Value::Uint32(l), Value::Uint32(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Uint32(l % r)
                        }
                        (Value::Uint64(l), Value::Uint64(r)) => {
                            if r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            Value::Uint64(l % r)
                        }
                        (l, r) => {
                            return Err(VmError::TypeMismatch {
                                expected: "matching integer types".to_string(),
                                found: format!("{:?} and {:?}", l, r),
                            });
                        }
                    };
                    self.write_reg(dest, res)?;
                }
                Instruction::Pow { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    let res = self.pow_values(lhs_val, rhs_val)?;
                    self.write_reg(dest, res)?;
                }
                Instruction::Neg { dest, src } => {
                    let val = self.read_reg(src)?;
                    let res = match val {
                        Value::Int8(x) => Value::Int8(x.wrapping_neg()),
                        Value::Int16(x) => Value::Int16(x.wrapping_neg()),
                        Value::Int32(x) => Value::Int32(x.wrapping_neg()),
                        Value::Int64(x) => Value::Int64(x.wrapping_neg()),
                        Value::Float32(x) => Value::Float32(-x),
                        Value::Float64(x) => Value::Float64(-x),
                        x => {
                            return Err(VmError::TypeMismatch {
                                expected: "signed numeric type".to_string(),
                                found: format!("{:?}", x),
                            });
                        }
                    };
                    self.write_reg(dest, res)?;
                }
                Instruction::Not { dest, src } => {
                    let val = self.read_reg(src)?;
                    let res = match val {
                        Value::Bool(b) => Value::Bool(!b),
                        x => {
                            return Err(VmError::TypeMismatch {
                                expected: "boolean type".to_string(),
                                found: format!("{:?}", x),
                            });
                        }
                    };
                    self.write_reg(dest, res)?;
                }
                Instruction::BitNot { dest, src } => {
                    let val = self.read_reg(src)?;
                    let res = match val {
                        Value::Int8(x) => Value::Int8(!x),
                        Value::Int16(x) => Value::Int16(!x),
                        Value::Int32(x) => Value::Int32(!x),
                        Value::Int64(x) => Value::Int64(!x),
                        Value::Uint8(x) => Value::Uint8(!x),
                        Value::Uint16(x) => Value::Uint16(!x),
                        Value::Uint32(x) => Value::Uint32(!x),
                        Value::Uint64(x) => Value::Uint64(!x),
                        x => {
                            return Err(VmError::TypeMismatch {
                                expected: "integer type".to_string(),
                                found: format!("{:?}", x),
                            });
                        }
                    };
                    self.write_reg(dest, res)?;
                }
                Instruction::Shl { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    let shift = self.to_shift_amount(rhs_val)?;
                    let res = match lhs_val {
                        Value::Int8(l) => Value::Int8(l.wrapping_shl(shift)),
                        Value::Int16(l) => Value::Int16(l.wrapping_shl(shift)),
                        Value::Int32(l) => Value::Int32(l.wrapping_shl(shift)),
                        Value::Int64(l) => Value::Int64(l.wrapping_shl(shift)),
                        Value::Uint8(l) => Value::Uint8(l.wrapping_shl(shift)),
                        Value::Uint16(l) => Value::Uint16(l.wrapping_shl(shift)),
                        Value::Uint32(l) => Value::Uint32(l.wrapping_shl(shift)),
                        Value::Uint64(l) => Value::Uint64(l.wrapping_shl(shift)),
                        l => {
                            return Err(VmError::TypeMismatch {
                                expected: "integer type for shift".to_string(),
                                found: format!("{:?}", l),
                            });
                        }
                    };
                    self.write_reg(dest, res)?;
                }
                Instruction::Shr { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    let shift = self.to_shift_amount(rhs_val)?;
                    let res = match lhs_val {
                        Value::Int8(l) => Value::Int8(l.wrapping_shr(shift)),
                        Value::Int16(l) => Value::Int16(l.wrapping_shr(shift)),
                        Value::Int32(l) => Value::Int32(l.wrapping_shr(shift)),
                        Value::Int64(l) => Value::Int64(l.wrapping_shr(shift)),
                        Value::Uint8(l) => Value::Uint8(l.wrapping_shr(shift)),
                        Value::Uint16(l) => Value::Uint16(l.wrapping_shr(shift)),
                        Value::Uint32(l) => Value::Uint32(l.wrapping_shr(shift)),
                        Value::Uint64(l) => Value::Uint64(l.wrapping_shr(shift)),
                        l => {
                            return Err(VmError::TypeMismatch {
                                expected: "integer type for shift".to_string(),
                                found: format!("{:?}", l),
                            });
                        }
                    };
                    self.write_reg(dest, res)?;
                }
                Instruction::And { dest, lhs, rhs } => {
                    impl_bitwise_op!(self, dest, lhs, rhs, &);
                }
                Instruction::Or { dest, lhs, rhs } => {
                    impl_bitwise_op!(self, dest, lhs, rhs, |);
                }
                Instruction::Xor { dest, lhs, rhs } => {
                    impl_bitwise_op!(self, dest, lhs, rhs, ^);
                }
                Instruction::Eq { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    self.write_reg(dest, Value::Bool(lhs_val == rhs_val))?;
                }
                Instruction::Ne { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    self.write_reg(dest, Value::Bool(lhs_val != rhs_val))?;
                }
                Instruction::Lt { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    let cmp = self.compare_values(&lhs_val, &rhs_val)?;
                    self.write_reg(dest, Value::Bool(cmp.is_some_and(|o| o.is_lt())))?;
                }
                Instruction::Le { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    let cmp = self.compare_values(&lhs_val, &rhs_val)?;
                    self.write_reg(dest, Value::Bool(cmp.is_some_and(|o| o.is_le())))?;
                }
                Instruction::Gt { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    let cmp = self.compare_values(&lhs_val, &rhs_val)?;
                    self.write_reg(dest, Value::Bool(cmp.is_some_and(|o| o.is_gt())))?;
                }
                Instruction::Ge { dest, lhs, rhs } => {
                    let lhs_val = self.read_reg(lhs)?;
                    let rhs_val = self.read_reg(rhs)?;
                    let cmp = self.compare_values(&lhs_val, &rhs_val)?;
                    self.write_reg(dest, Value::Bool(cmp.is_some_and(|o| o.is_ge())))?;
                }
                Instruction::Fallback {
                    dest,
                    src,
                    fallback,
                } => {
                    let src_val = self.read_reg(src)?;
                    let fallback_val = self.read_reg(fallback)?;
                    let val = match src_val {
                        Value::Null => fallback_val,
                        _ => src_val,
                    };
                    self.write_reg(dest, val)?;
                }

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
                        return Ok(val);
                    } else {
                        // The callee PC was already advanced by 1 in Call,
                        // so we need to fetch the Call instruction to know where the destination register is!
                        let frame = self.call_stack.last().ok_or(VmError::EmptyCallStack)?;
                        let func = &self.image.functions[frame.func_idx.raw() as usize];
                        let call_pc = frame.pc - 1;
                        if let Some(Instruction::Call { dest, .. }) = func.instructions.get(call_pc)
                        {
                            self.write_reg(*dest, val)?;
                        } else {
                            return Err(VmError::InvalidJumpTarget { pc });
                        }
                    }
                }
                Instruction::RetNull => {
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        return Ok(Value::Null);
                    } else {
                        let frame = self.call_stack.last().ok_or(VmError::EmptyCallStack)?;
                        let func = &self.image.functions[frame.func_idx.raw() as usize];
                        let call_pc = frame.pc - 1;
                        if let Some(Instruction::Call { dest, .. }) = func.instructions.get(call_pc)
                        {
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

                // Category D: Heaps, Structs & Collections
                Instruction::AllocLocal { dest, type_idx }
                | Instruction::AllocShared { dest, type_idx } => {
                    let ty = self
                        .image
                        .types
                        .get(type_idx.raw() as usize)
                        .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                    if let ImageType::Struct(layout_idx) = ty {
                        let layout = self
                            .image
                            .struct_layouts
                            .get(layout_idx.raw() as usize)
                            .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                        let fields = vec![Value::Null; layout.fields.len()];
                        let obj_ref = self.alloc(HeapObject::Struct {
                            layout_idx: *layout_idx,
                            fields,
                        });
                        self.write_reg(dest, Value::Object(obj_ref))?;
                    } else {
                        return Err(VmError::TypeMismatch {
                            expected: "Struct type".to_string(),
                            found: format!("{:?}", ty),
                        });
                    }
                }
                Instruction::LoadField { dest, obj, field } => {
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
                Instruction::StoreField { obj, field, val } => {
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
                Instruction::NewArray {
                    dest,
                    type_idx,
                    len_reg,
                } => {
                    let ty = self
                        .image
                        .types
                        .get(type_idx.raw() as usize)
                        .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                    let element_ty = match ty {
                        ImageType::Array(el_ty) => *el_ty,
                        ImageType::FixedArray(el_ty, _) => *el_ty,
                        _ => {
                            return Err(VmError::TypeMismatch {
                                expected: "Array or FixedArray type".to_string(),
                                found: format!("{:?}", ty),
                            });
                        }
                    };
                    let len_val = self.read_reg(len_reg)?;
                    let len = match len_val {
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
                                expected: "positive array length".to_string(),
                                found: format!("{:?}", x),
                            });
                        }
                    };
                    let elements = vec![Value::Null; len];
                    let obj_ref = self.alloc(HeapObject::Array {
                        element_ty,
                        elements,
                    });
                    self.write_reg(dest, Value::Object(obj_ref))?;
                }
                Instruction::LoadIndex { dest, arr, idx } => {
                    let arr_val = self.read_reg(arr)?;
                    let idx_val = self.read_reg(idx)?;
                    let index = self.to_array_index(idx_val)?;
                    if let Value::Object(obj_ref) = arr_val {
                        let heap_obj = self.get_object(obj_ref)?;
                        let val = match heap_obj {
                            HeapObject::Array { elements, .. } | HeapObject::Tuple { elements } => {
                                elements
                                    .get(index)
                                    .cloned()
                                    .ok_or(VmError::IndexOutOfBounds {
                                        index,
                                        len: elements.len(),
                                    })?
                            }
                            _ => {
                                return Err(VmError::TypeMismatch {
                                    expected: "Array or Tuple object".to_string(),
                                    found: format!("{:?}", heap_obj),
                                });
                            }
                        };
                        self.write_reg(dest, val)?;
                    } else {
                        return Err(VmError::TypeMismatch {
                            expected: "Object reference".to_string(),
                            found: format!("{:?}", arr_val),
                        });
                    }
                }
                Instruction::StoreIndex { arr, idx, val } => {
                    let arr_val = self.read_reg(arr)?;
                    let idx_val = self.read_reg(idx)?;
                    let index = self.to_array_index(idx_val)?;
                    let val_to_store = self.read_reg(val)?;
                    if let Value::Object(obj_ref) = arr_val {
                        let heap_obj = self.get_object_mut(obj_ref)?;
                        match heap_obj {
                            HeapObject::Array { elements, .. } | HeapObject::Tuple { elements } => {
                                if index < elements.len() {
                                    elements[index] = val_to_store;
                                } else {
                                    return Err(VmError::IndexOutOfBounds {
                                        index,
                                        len: elements.len(),
                                    });
                                }
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
                            found: format!("{:?}", arr_val),
                        });
                    }
                }
                Instruction::NewTuple {
                    dest,
                    type_idx,
                    start,
                    count,
                } => {
                    let ty = self
                        .image
                        .types
                        .get(type_idx.raw() as usize)
                        .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                    if let ImageType::Tuple(tys) = ty {
                        if count as usize != tys.len() {
                            return Err(VmError::TypeMismatch {
                                expected: format!("Tuple size {}", tys.len()),
                                found: format!("Tuple size {}", count),
                            });
                        }
                        let mut elements = Vec::new();
                        for i in 0..count as usize {
                            let src_reg = Reg(start.raw() + i as u16);
                            elements.push(self.read_reg(src_reg)?);
                        }
                        let obj_ref = self.alloc(HeapObject::Tuple { elements });
                        self.write_reg(dest, Value::Object(obj_ref))?;
                    } else {
                        return Err(VmError::TypeMismatch {
                            expected: "Tuple type".to_string(),
                            found: format!("{:?}", ty),
                        });
                    }
                }
                Instruction::NewChoice {
                    dest,
                    type_idx,
                    variant_idx,
                    payload,
                } => {
                    let ty = self
                        .image
                        .types
                        .get(type_idx.raw() as usize)
                        .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                    if let ImageType::Choice(layout_idx) = ty {
                        let payload_val = self.read_reg(payload)?;
                        let obj_ref = self.alloc(HeapObject::Choice {
                            layout_idx: *layout_idx,
                            variant_idx,
                            payload: payload_val,
                        });
                        self.write_reg(dest, Value::Object(obj_ref))?;
                    } else {
                        return Err(VmError::TypeMismatch {
                            expected: "Choice type".to_string(),
                            found: format!("{:?}", ty),
                        });
                    }
                }
                Instruction::Cast {
                    dest,
                    src,
                    type_idx,
                } => {
                    let val = self.read_reg(src)?;
                    let res = self.cast_value(&val, type_idx)?;
                    self.write_reg(dest, res)?;
                }
                Instruction::Instanceof {
                    dest,
                    src,
                    type_idx,
                } => {
                    let val = self.read_reg(src)?;
                    let is_instance = self.check_value_type(&val, type_idx);
                    self.write_reg(dest, Value::Bool(is_instance))?;
                }

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
                Instruction::Len { dest, src } => {
                    let val = self.read_reg(src)?;
                    if let Value::Object(obj_ref) = val {
                        let heap_obj = self.get_object(obj_ref)?;
                        match heap_obj {
                            HeapObject::Array { elements, .. } => {
                                self.write_reg(dest, Value::Int64(elements.len() as i64))?;
                            }
                            HeapObject::Tuple { elements, .. } => {
                                self.write_reg(dest, Value::Int64(elements.len() as i64))?;
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
            }
            self.release_unreachable();
        }
    }

    fn get_object(&self, obj_ref: ObjectRef) -> Result<&HeapObject, VmError> {
        self.heap
            .get(obj_ref.raw())
            .and_then(|opt| opt.as_ref())
            .ok_or_else(|| VmError::TypeMismatch {
                expected: "valid object reference".to_string(),
                found: format!("{:?}", obj_ref),
            })
    }

    fn get_object_mut(&mut self, obj_ref: ObjectRef) -> Result<&mut HeapObject, VmError> {
        self.heap
            .get_mut(obj_ref.raw())
            .and_then(|opt| opt.as_mut())
            .ok_or_else(|| VmError::TypeMismatch {
                expected: "valid object reference".to_string(),
                found: format!("{:?}", obj_ref),
            })
    }

    fn to_shift_amount(&self, val: Value) -> Result<u32, VmError> {
        match val {
            Value::Int8(x) => Ok(x as u32),
            Value::Int16(x) => Ok(x as u32),
            Value::Int32(x) => Ok(x as u32),
            Value::Int64(x) => Ok(x as u32),
            Value::Uint8(x) => Ok(x as u32),
            Value::Uint16(x) => Ok(x as u32),
            Value::Uint32(x) => Ok(x),
            Value::Uint64(x) => Ok(x as u32),
            x => Err(VmError::TypeMismatch {
                expected: "integer shift amount".to_string(),
                found: format!("{:?}", x),
            }),
        }
    }

    fn to_array_index(&self, val: Value) -> Result<usize, VmError> {
        match val {
            Value::Int8(x) if x >= 0 => Ok(x as usize),
            Value::Int16(x) if x >= 0 => Ok(x as usize),
            Value::Int32(x) if x >= 0 => Ok(x as usize),
            Value::Int64(x) if x >= 0 => Ok(x as usize),
            Value::Uint8(x) => Ok(x as usize),
            Value::Uint16(x) => Ok(x as usize),
            Value::Uint32(x) => Ok(x as usize),
            Value::Uint64(x) => Ok(x as usize),
            x => Err(VmError::TypeMismatch {
                expected: "valid non-negative array index".to_string(),
                found: format!("{:?}", x),
            }),
        }
    }

    fn pow_values(&self, lhs: Value, rhs: Value) -> Result<Value, VmError> {
        let res = match (lhs, rhs) {
            (Value::Int8(l), Value::Int8(r)) => Value::Int8(l.wrapping_pow(r as u32)),
            (Value::Int16(l), Value::Int16(r)) => Value::Int16(l.wrapping_pow(r as u32)),
            (Value::Int32(l), Value::Int32(r)) => Value::Int32(l.wrapping_pow(r as u32)),
            (Value::Int64(l), Value::Int64(r)) => Value::Int64(l.wrapping_pow(r as u32)),
            (Value::Uint8(l), Value::Uint8(r)) => Value::Uint8(l.wrapping_pow(r as u32)),
            (Value::Uint16(l), Value::Uint16(r)) => Value::Uint16(l.wrapping_pow(r as u32)),
            (Value::Uint32(l), Value::Uint32(r)) => Value::Uint32(l.wrapping_pow(r)),
            (Value::Uint64(l), Value::Uint64(r)) => Value::Uint64(l.wrapping_pow(r as u32)),
            (Value::Float32(l), Value::Float32(r)) => Value::Float32(l.powf(r)),
            (Value::Float64(l), Value::Float64(r)) => Value::Float64(l.powf(r)),
            (l, r) => {
                return Err(VmError::TypeMismatch {
                    expected: "matching numeric types".to_string(),
                    found: format!("{:?} and {:?}", l, r),
                });
            }
        };
        Ok(res)
    }

    fn compare_values(
        &self,
        lhs: &Value,
        rhs: &Value,
    ) -> Result<Option<std::cmp::Ordering>, VmError> {
        let ord = match (lhs, rhs) {
            (Value::Bool(l), Value::Bool(r)) => Some(l.cmp(r)),
            (Value::Int8(l), Value::Int8(r)) => Some(l.cmp(r)),
            (Value::Int16(l), Value::Int16(r)) => Some(l.cmp(r)),
            (Value::Int32(l), Value::Int32(r)) => Some(l.cmp(r)),
            (Value::Int64(l), Value::Int64(r)) => Some(l.cmp(r)),
            (Value::Uint8(l), Value::Uint8(r)) => Some(l.cmp(r)),
            (Value::Uint16(l), Value::Uint16(r)) => Some(l.cmp(r)),
            (Value::Uint32(l), Value::Uint32(r)) => Some(l.cmp(r)),
            (Value::Uint64(l), Value::Uint64(r)) => Some(l.cmp(r)),
            (Value::Float32(l), Value::Float32(r)) => l.partial_cmp(r),
            (Value::Float64(l), Value::Float64(r)) => l.partial_cmp(r),
            (l, r) => {
                return Err(VmError::TypeMismatch {
                    expected: "matching comparable types".to_string(),
                    found: format!("{:?} and {:?}", l, r),
                });
            }
        };
        Ok(ord)
    }

    fn check_value_type(&self, val: &Value, expected_ty: TypeIdx) -> bool {
        let ty = match self.image.types.get(expected_ty.raw() as usize) {
            Some(t) => t,
            None => return false,
        };
        match (val, ty) {
            (Value::Null, ImageType::Null) => true,
            (Value::Bool(_), ImageType::Bool) => true,
            (Value::Int8(_), ImageType::Int8) => true,
            (Value::Int16(_), ImageType::Int16) => true,
            (Value::Int32(_), ImageType::Int32) => true,
            (Value::Int64(_), ImageType::Int64) => true,
            (Value::Uint8(_), ImageType::Uint8) => true,
            (Value::Uint16(_), ImageType::Uint16) => true,
            (Value::Uint32(_), ImageType::Uint32) => true,
            (Value::Uint64(_), ImageType::Uint64) => true,
            (Value::Float32(_), ImageType::Float32) => true,
            (Value::Float64(_), ImageType::Float64) => true,
            (Value::Object(obj_ref), ImageType::Struct(expected_layout_idx)) => {
                if let Ok(HeapObject::Struct { layout_idx, .. }) = self.get_object(*obj_ref) {
                    layout_idx == expected_layout_idx
                } else {
                    false
                }
            }
            (Value::Object(obj_ref), ImageType::Array(expected_el_ty)) => {
                if let Ok(HeapObject::Array { element_ty, .. }) = self.get_object(*obj_ref) {
                    element_ty == expected_el_ty
                } else {
                    false
                }
            }
            (Value::Object(obj_ref), ImageType::FixedArray(expected_el_ty, expected_len)) => {
                if let Ok(HeapObject::Array {
                    element_ty,
                    elements,
                }) = self.get_object(*obj_ref)
                {
                    element_ty == expected_el_ty && elements.len() == *expected_len
                } else {
                    false
                }
            }
            (Value::Object(obj_ref), ImageType::Tuple(expected_tys)) => {
                if let Ok(HeapObject::Tuple { elements }) = self.get_object(*obj_ref) {
                    if elements.len() == expected_tys.len() {
                        elements
                            .iter()
                            .zip(expected_tys.iter())
                            .all(|(v, &ty)| self.check_value_type(v, ty))
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            (Value::Object(obj_ref), ImageType::Choice(expected_layout_idx)) => {
                if let Ok(HeapObject::Choice { layout_idx, .. }) = self.get_object(*obj_ref) {
                    layout_idx == expected_layout_idx
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn cast_value(&self, val: &Value, target_ty: TypeIdx) -> Result<Value, VmError> {
        let ty = self
            .image
            .types
            .get(target_ty.raw() as usize)
            .ok_or(VmError::TypeOutOfBounds { index: target_ty })?;

        if self.check_value_type(val, target_ty) {
            return Ok(val.clone());
        }

        // Numeric casting / conversions
        let casted = match val {
            Value::Int8(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Int16(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Int32(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Int64(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Uint8(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Uint16(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Uint32(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Uint64(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Float32(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Float64(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x)),
                _ => None,
            },
            _ => None,
        };

        casted.ok_or_else(|| VmError::TypeMismatch {
            expected: format!("{:?}", ty),
            found: format!("{:?}", val),
        })
    }

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

    fn execute_write(&mut self, val: Value) -> Result<(), VmError> {
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
        self.io_handler.write(&bytes)?;
        Ok(())
    }
}
