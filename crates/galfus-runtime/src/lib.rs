use galfus_bytecode::BytecodeModule;
use galfus_host::Providers;
use galfus_vm::{HeapObject, VirtualMachine, VmPanic, VmValue};

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("module `{0}` is not loaded")]
    ModuleNotLoaded(String),
    #[error("entry function `{0}` is not exported by the entry module")]
    EntryNotExported(String),
    #[error("entry function `{name}` expects {expected} parameter(s), found {found}")]
    EntryArityMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    #[error("entry function `{name}` must return i32")]
    EntryReturnTypeMismatch { name: String },
    #[error("entry arguments require bytecode type `{0}`")]
    MissingArgumentType(&'static str),
    #[error(transparent)]
    GraphResolution(#[from] galfus_bytecode::GraphResolutionError),
    #[error("{0}")]
    VmPanic(#[from] VmPanic),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryArgsType {
    ByteArgv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryReturnType {
    Int32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryAbi {
    pub args_type: EntryArgsType,
    pub return_type: EntryReturnType,
}

impl EntryAbi {
    pub const fn default_app() -> Self {
        Self {
            args_type: EntryArgsType::ByteArgv,
            return_type: EntryReturnType::Int32,
        }
    }

    fn expected_param_count(self) -> u8 {
        match self.args_type {
            EntryArgsType::ByteArgv => 1,
        }
    }

    fn accepts_return_type(self, ty: &galfus_bytecode::BytecodeType) -> bool {
        match self.return_type {
            EntryReturnType::Int32 => ty == &galfus_bytecode::BytecodeType::Int32,
        }
    }
}

/// A single execution composed from one executable graph and optional host providers.
pub struct Runtime<'graph> {
    graph: &'graph galfus_bytecode::BytecodeGraph,
    providers: Option<Providers>,
}

impl<'graph> Runtime<'graph> {
    pub fn new(
        graph: &'graph galfus_bytecode::BytecodeGraph,
        providers: Option<Providers>,
    ) -> Self {
        Self { graph, providers }
    }

    /// Execute an entry exported by a module loaded in the given BytecodeGraph.
    pub fn run_module_entry(
        self,
        module_id: galfus_core::ModuleId,
        entry_name: &str,
        args: &[Vec<u8>],
    ) -> Result<i32, RuntimeError> {
        let graph = self.graph;
        let image = &graph.get(module_id).unwrap().module;
        let abi = EntryAbi::default_app();
        let entry_idx = image
            .exports
            .iter()
            .find(|export| export.symbol_name == entry_name)
            .map(|export| export.func_idx)
            .ok_or_else(|| RuntimeError::EntryNotExported(entry_name.to_string()))?;

        let entry_func = &image.functions[entry_idx.raw() as usize];
        if entry_func.param_count != abi.expected_param_count() {
            return Err(RuntimeError::EntryArityMismatch {
                name: entry_name.to_string(),
                expected: abi.expected_param_count() as usize,
                found: entry_func.param_count as usize,
            });
        }
        let return_ty = image.types.get(entry_func.return_ty.raw() as usize);
        if !return_ty.is_some_and(|ty| abi.accepts_return_type(ty)) {
            return Err(RuntimeError::EntryReturnTypeMismatch {
                name: entry_name.to_string(),
            });
        }

        let mut vm = VirtualMachine::new(graph).with_providers(self.providers);

        let result = (|| {
            for initialized_module_id in graph.initialization_order(module_id)? {
                if vm.is_module_initialized(initialized_module_id) {
                    continue;
                }
                if let Some(init_idx) = graph
                    .get(initialized_module_id)
                    .expect("initialization order only contains loaded modules")
                    .module
                    .init_func_idx
                {
                    vm.run_function(initialized_module_id, init_idx, vec![])?;
                }
                vm.mark_module_initialized(initialized_module_id);
            }

            let entry_args = build_entry_args(&mut vm, module_id, args)?;
            vm.run_function(module_id, entry_idx, vec![entry_args])
                .map_err(RuntimeError::VmPanic)
        })();
        let result = result?;

        match result {
            galfus_vm::VmValue::Int32(code) => Ok(code),
            galfus_vm::VmValue::Null => Ok(0),
            _other => Err(RuntimeError::EntryReturnTypeMismatch {
                name: entry_name.to_string(),
            }),
        }
    }
}

fn build_entry_args(
    vm: &mut VirtualMachine,
    module_id: galfus_core::ModuleId,
    args: &[Vec<u8>],
) -> Result<VmValue, RuntimeError> {
    let uint8_ty = find_type(&vm.graph.get(module_id).unwrap().module, |ty| {
        matches!(ty, galfus_bytecode::BytecodeType::Uint8)
    })
    .ok_or(RuntimeError::MissingArgumentType("u8"))?;
    let byte_array_ty = vm
        .graph.get(module_id).unwrap().module
        .types
        .iter()
        .enumerate()
        .find(|(_, ty)| {
            matches!(ty, galfus_bytecode::BytecodeType::Array(element)
                if matches!(vm.graph.get(module_id).unwrap().module.types.get(element.raw() as usize), Some(galfus_bytecode::BytecodeType::Uint8)))
        })
        .map(|(index, _)| galfus_bytecode::instruction::TypeIdx(index as u16))
        .ok_or(RuntimeError::MissingArgumentType("[u8]"))?;
    let args_array_ty = vm
        .graph.get(module_id).unwrap().module
        .types
        .iter()
        .enumerate()
        .find(|(_, ty)| {
            matches!(ty, galfus_bytecode::BytecodeType::Array(element)
                if matches!(vm.graph.get(module_id).unwrap().module.types.get(element.raw() as usize), Some(galfus_bytecode::BytecodeType::Array(inner))
                    if matches!(vm.graph.get(module_id).unwrap().module.types.get(inner.raw() as usize), Some(galfus_bytecode::BytecodeType::Uint8))))
        })
        .map(|(index, _)| galfus_bytecode::instruction::TypeIdx(index as u16))
        .ok_or(RuntimeError::MissingArgumentType("[[u8]]"))?;

    let mut arg_values = Vec::with_capacity(args.len());
    for arg in args {
        let elements = arg.iter().copied().map(VmValue::Uint8).collect();
        let arg_ref = vm.alloc(HeapObject::Array {
            element_ty: uint8_ty,
            elements,
        });
        arg_values.push(VmValue::Object(arg_ref));
    }

    let args_ref = vm.alloc(HeapObject::Array {
        element_ty: byte_array_ty,
        elements: arg_values,
    });

    let _args_array_ty = args_array_ty;
    Ok(VmValue::Object(args_ref))
}

fn find_type(
    module: &BytecodeModule,
    predicate: impl Fn(&galfus_bytecode::BytecodeType) -> bool,
) -> Option<galfus_bytecode::instruction::TypeIdx> {
    module
        .types
        .iter()
        .position(predicate)
        .map(|index| galfus_bytecode::instruction::TypeIdx(index as u16))
}

pub fn format_panic(graph: &galfus_bytecode::BytecodeGraph, panic: &VmPanic) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    writeln!(&mut out, "Runtime Panic: {}", panic.error).unwrap();
    writeln!(&mut out, "Stack trace:").unwrap();

    for (i, frame) in panic.stack_trace.iter().enumerate() {
        if let Some(module) = graph.get(frame.module_id) {
            let func_name = module
                .module
                .functions
                .get(frame.func_idx.raw() as usize)
                .map(|f| f.name.as_str())
                .unwrap_or("<unknown>");

            // Check metadata if available
            let mut location_str = format!("PC {}", frame.pc);

            // Currently ExecutionMetadata is not populated, but we prepare for it
            if let Some(metadata) = &module.metadata
                && let Some(func_spans) = metadata.spans.get(&frame.func_idx)
                && let Some(span) = func_spans.get(&frame.pc)
            {
                // Assuming source file resolution would be passed in,
                // for now we just show the raw span info or we can ignore it.
                location_str = format!("PC {}, Span {:?}", frame.pc, span);
            }

            writeln!(
                &mut out,
                "  #{}: {}::{} (at {})",
                i,
                module.path.as_str(),
                func_name,
                location_str
            )
            .unwrap();
        } else {
            writeln!(
                &mut out,
                "  #{}: Module {:?} Func {:?} (at PC {})",
                i, frame.module_id, frame.func_idx, frame.pc
            )
            .unwrap();
        }
    }

    out
}
