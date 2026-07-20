use crate::error::{StackFrameInfo, VmError, VmPanic};
use galfus_bytecode::instruction::{
    ChoiceLayoutIdx, FuncIdx, Instruction, Reg, StructLayoutIdx, TypeIdx,
};
use galfus_bytecode::{BytecodeModule, Constant, ImageType, OwnershipKind};
use galfus_host::Providers;

mod casts;
mod control;
mod data;
mod graph_release;
mod heap;
mod objects;
mod operators;
mod system;
mod target_io;
#[cfg(test)]
mod tests;

pub(super) enum ExecutionStep {
    Continue,
    Return(Value),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VmObjectRef(pub usize);

impl VmObjectRef {
    pub const fn raw(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VmValue {
    Null,
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Uint64(u64),
    Float32(f32),
    Float64(f64),
    Object(VmObjectRef),
    Function(FuncIdx),
}

type Value = VmValue;
type ObjectRef = VmObjectRef;

const RELEASE_ALLOCATION_THRESHOLD: usize = 64;

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

pub struct VmContext {
    providers: Option<Providers>,
}

impl VmContext {
    pub fn new(providers: Option<Providers>) -> Self {
        Self { providers }
    }
}

pub struct CallFrame {
    pub func_idx: FuncIdx,
    pub pc: usize,
    pub registers: Vec<Value>,
    pub return_dest: Option<Reg>,
    pub in_transaction: bool,
}

pub struct VirtualMachine {
    pub image: BytecodeModule,
    pub globals: Vec<Value>,
    pub heap: Vec<Option<HeapObject>>,
    pub free_slots: Vec<usize>,
    pub call_stack: Vec<CallFrame>,
    pub context: VmContext,
    allocations_since_release: usize,
}

impl VirtualMachine {
    pub fn new(image: BytecodeModule) -> Self {
        Self {
            image,
            globals: Vec::new(),
            heap: Vec::new(),
            free_slots: Vec::new(),
            call_stack: Vec::new(),
            context: VmContext::new(None),
            allocations_since_release: 0,
        }
    }

    pub fn with_context(mut self, context: VmContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_providers(mut self, providers: Option<Providers>) -> Self {
        self.context = VmContext::new(providers);
        self
    }

    pub fn alloc(&mut self, obj: HeapObject) -> ObjectRef {
        self.allocations_since_release += 1;

        if let Some(idx) = self.free_slots.pop() {
            self.heap[idx] = Some(obj);
            VmObjectRef(idx)
        } else {
            let idx = self.heap.len();
            self.heap.push(Some(obj));
            VmObjectRef(idx)
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
            return_dest: None,
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
            let instr = {
                let frame = self.call_stack.last_mut().ok_or(VmError::EmptyCallStack)?;
                let func = &self.image.functions[frame.func_idx.raw() as usize];
                if frame.pc >= func.instructions.len() {
                    return Err(VmError::InstructionPointerOutOfBounds { pc: frame.pc });
                }
                let instr = func.instructions[frame.pc];
                frame.pc += 1;
                instr
            };

            let step = match instr {
                Instruction::LoadConst { .. }
                | Instruction::Move { .. }
                | Instruction::LoadGlobal { .. }
                | Instruction::StoreGlobal { .. }
                | Instruction::LoadNull { .. } => self.execute_data_instruction(instr)?,

                Instruction::Add { .. }
                | Instruction::Sub { .. }
                | Instruction::Mul { .. }
                | Instruction::Div { .. }
                | Instruction::Rem { .. }
                | Instruction::Pow { .. }
                | Instruction::Neg { .. }
                | Instruction::Not { .. }
                | Instruction::BitNot { .. }
                | Instruction::Shl { .. }
                | Instruction::Shr { .. }
                | Instruction::And { .. }
                | Instruction::Or { .. }
                | Instruction::Xor { .. }
                | Instruction::Eq { .. }
                | Instruction::Ne { .. }
                | Instruction::Lt { .. }
                | Instruction::Le { .. }
                | Instruction::Gt { .. }
                | Instruction::Ge { .. }
                | Instruction::Fallback { .. } => self.execute_operator_instruction(instr)?,

                Instruction::Jump { .. }
                | Instruction::JumpTrue { .. }
                | Instruction::JumpFalse { .. }
                | Instruction::JumpNull { .. }
                | Instruction::Call { .. }
                | Instruction::CallMethod { .. }
                | Instruction::CallDynamic { .. }
                | Instruction::Ret { .. }
                | Instruction::RetNull
                | Instruction::Panic { .. } => self.execute_control_instruction(instr)?,

                Instruction::AllocLocal { .. }
                | Instruction::AllocShared { .. }
                | Instruction::LoadField { .. }
                | Instruction::StoreField { .. }
                | Instruction::NewArray { .. }
                | Instruction::LoadIndex { .. }
                | Instruction::StoreIndex { .. }
                | Instruction::NewTuple { .. }
                | Instruction::NewChoice { .. }
                | Instruction::Cast { .. }
                | Instruction::Copy { .. }
                | Instruction::Instanceof { .. } => self.execute_object_instruction(instr)?,

                Instruction::Drop { .. }
                | Instruction::TxStart { .. }
                | Instruction::TxLoad { .. }
                | Instruction::TxStore { .. }
                | Instruction::TxCommit { .. }
                | Instruction::TxRollback
                | Instruction::Write { .. }
                | Instruction::Read { .. }
                | Instruction::Len { .. }
                | Instruction::CopyArray { .. } => self.execute_system_instruction(instr)?,
            };

            match step {
                ExecutionStep::Continue => self.release_unreachable_if_needed(instr),
                ExecutionStep::Return(value) => return Ok(value),
            }
        }
    }

    fn release_unreachable_if_needed(&mut self, instr: Instruction) {
        if matches!(instr, Instruction::Drop { .. })
            || self.allocations_since_release >= RELEASE_ALLOCATION_THRESHOLD
        {
            self.release_unreachable();
        }
    }
}
