use crate::error::{StackFrameInfo, VmError, VmPanic};
use galfus_bytecode::instruction::{
    ChoiceLayoutIdx, FuncIdx, Instruction, Reg, StructLayoutIdx, TypeIdx,
};
use galfus_bytecode::{BytecodeGraph, BytecodeType, Constant, OwnershipKind};
use galfus_contract::Providers;
use galfus_core::ModuleId;
use std::cell::RefCell;
use std::collections::HashMap;

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

pub enum ExecutionStep {
    Continue,
    Return(Value),
    Blocked,
    SendMsg { target: u64, msg: Value },
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

pub type Value = VmValue;
type ObjectRef = VmObjectRef;

const RELEASE_ALLOCATION_THRESHOLD: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub enum HeapObject {
    Struct {
        module_id: galfus_core::ModuleId,
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
        module_id: galfus_core::ModuleId,
        layout_idx: ChoiceLayoutIdx,
        variant_idx: u16,
        payload: Value,
    },
}

pub struct VmContext {
    providers: Option<RefCell<Providers>>,
}

#[derive(Default)]
pub struct RuntimeModuleState {
    pub globals: Vec<VmValue>,
    pub initialized: bool,
}

impl VmContext {
    pub fn new(providers: Option<Providers>) -> Self {
        Self {
            providers: providers.map(RefCell::new),
        }
    }
}

pub struct CallFrame {
    pub module_id: ModuleId,
    pub func_idx: FuncIdx,
    pub pc: usize,
    pub registers: Vec<Value>,
    pub return_dest: Option<Reg>,
}

pub struct VirtualMachine<'a> {
    pub graph: &'a BytecodeGraph,
    pub context: VmContext,
}

impl<'a> VirtualMachine<'a> {
    pub fn new(graph: &'a BytecodeGraph) -> Self {
        Self {
            graph,
            context: VmContext::new(None),
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

    pub fn current_image(
        &self,
        thread: &crate::thread::VirtualThread,
    ) -> Result<&'a galfus_bytecode::BytecodeModule, VmError> {
        let frame = thread.call_stack.last().ok_or(VmError::EmptyCallStack)?;
        Ok(&self.graph.get(frame.module_id).unwrap().module)
    }

    pub fn run_function(
        &self,
        thread: &mut crate::thread::VirtualThread,
        module_id: galfus_core::ModuleId,
        func_idx: FuncIdx,
        args: Vec<Value>,
    ) -> Result<Value, VmPanic> {
        if (func_idx.raw() as usize) >= self.graph.get(module_id).unwrap().module.functions.len() {
            return Err(VmPanic {
                error: VmError::FunctionOutOfBounds { index: func_idx },
                stack_trace: vec![],
            });
        }

        let func = &self.graph.get(module_id).unwrap().module.functions[func_idx.raw() as usize];
        if args.len() != func.param_count as usize {
            return Err(VmPanic {
                error: VmError::TypeMismatch {
                    expected: format!("{} arguments", func.param_count),
                    found: format!("{} arguments", args.len()),
                },
                stack_trace: vec![],
            });
        }

        thread.call_stack.clear();
        let total_regs =
            func.param_count as usize + func.local_count as usize + func.temp_count as usize;
        let mut registers = vec![Value::Null; total_regs];
        for (i, val) in args.into_iter().enumerate() {
            registers[i] = val;
        }

        thread.call_stack.push(CallFrame {
            module_id,
            func_idx,
            pc: 0,
            registers,
            return_dest: None,
        });

        match self.execute_loop(thread) {
            Ok(val) => Ok(val),
            Err(err) => {
                let mut stack_trace = Vec::new();
                for frame in thread.call_stack.iter().rev() {
                    stack_trace.push(StackFrameInfo {
                        module_id: frame.module_id,
                        func_idx: frame.func_idx,
                        instruction_offset: frame.pc.saturating_sub(1),
                    });
                }
                Err(VmPanic {
                    error: err,
                    stack_trace,
                })
            }
        }
    }

    pub fn execute_with_budget(
        &self,
        thread: &mut crate::thread::VirtualThread,
        mut budget: usize,
    ) -> Result<ExecutionStep, VmPanic> {
        while budget > 0 {
            match self.step(thread) {
                Ok(ExecutionStep::Continue) => budget -= 1,
                Ok(ExecutionStep::Return(val)) => return Ok(ExecutionStep::Return(val)),
                Ok(ExecutionStep::Blocked) => return Ok(ExecutionStep::Blocked),
                Ok(ExecutionStep::SendMsg { target, msg }) => {
                    return Ok(ExecutionStep::SendMsg { target, msg });
                }
                Err(err) => {
                    let mut stack_trace = Vec::new();
                    for frame in thread.call_stack.iter().rev() {
                        stack_trace.push(StackFrameInfo {
                            module_id: frame.module_id,
                            func_idx: frame.func_idx,
                            instruction_offset: frame.pc.saturating_sub(1),
                        });
                    }
                    return Err(VmPanic {
                        error: err,
                        stack_trace,
                    });
                }
            }
        }
        Ok(ExecutionStep::Continue)
    }

    pub fn step(
        &self,
        thread: &mut crate::thread::VirtualThread,
    ) -> Result<ExecutionStep, VmError> {
        let instr = {
            let frame = thread
                .call_stack
                .last_mut()
                .ok_or(VmError::EmptyCallStack)?;
            let func = &self.graph.get(frame.module_id).unwrap().module.functions
                [frame.func_idx.raw() as usize];
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
            | Instruction::LoadNull { .. } => self.execute_data_instruction(thread, instr)?,

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
            | Instruction::Fallback { .. } => self.execute_operator_instruction(thread, instr)?,

            Instruction::Jump { .. }
            | Instruction::JumpTrue { .. }
            | Instruction::JumpFalse { .. }
            | Instruction::JumpNull { .. }
            | Instruction::Call { .. }
            | Instruction::CallMethod { .. }
            | Instruction::CallDynamic { .. }
            | Instruction::Ret { .. }
            | Instruction::RetNull
            | Instruction::Send { .. }
            | Instruction::Receive { .. }
            | Instruction::Panic { .. } => self.execute_control_instruction(thread, instr)?,

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
            | Instruction::Instanceof { .. } => self.execute_object_instruction(thread, instr)?,

            Instruction::Drop { .. }
            | Instruction::Write { .. }
            | Instruction::Read { .. }
            | Instruction::Len { .. }
            | Instruction::CopyArray { .. } => self.execute_system_instruction(thread, instr)?,
        };

        if matches!(step, ExecutionStep::Continue) {
            self.release_unreachable_if_needed(thread, instr);
        }

        Ok(step)
    }

    fn execute_loop(&self, thread: &mut crate::thread::VirtualThread) -> Result<Value, VmError> {
        loop {
            match self.step(thread)? {
                ExecutionStep::Continue => {}
                ExecutionStep::Return(value) => return Ok(value),
                ExecutionStep::Blocked => return Err(VmError::UnresolvedHostBlocked),
                ExecutionStep::SendMsg { .. } => return Err(VmError::UnresolvedHostBlocked),
            }
        }
    }

    fn release_unreachable_if_needed(
        &self,
        thread: &mut crate::thread::VirtualThread,
        instr: Instruction,
    ) {
        if matches!(instr, Instruction::Drop { .. })
            || thread.heap.allocations_since_release >= RELEASE_ALLOCATION_THRESHOLD
        {
            self.release_unreachable(thread);
        }
    }
}
