use galfus_bytecode::BytecodeModule;
use galfus_host::Providers;
use galfus_vm::{HeapObject, VirtualMachine, VmPanic, VmValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(test)]
mod tests;

pub use galfus_bytecode::{LinkError, LinkedImport, ModuleLink};

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
    #[error("entry arguments require image type `{0}`")]
    MissingArgumentType(&'static str),
    #[error("{0}")]
    VmPanic(#[from] VmPanic),
    #[error(transparent)]
    Link(#[from] LinkError),
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

    fn accepts_return_type(self, ty: &galfus_bytecode::ImageType) -> bool {
        match self.return_type {
            EntryReturnType::Int32 => ty == &galfus_bytecode::ImageType::Int32,
        }
    }
}

pub struct ModuleRegistry {
    modules: HashMap<String, Arc<BytecodeModule>>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn register(&mut self, image: BytecodeModule) -> Arc<BytecodeModule> {
        let name = image.name.clone();
        let arc = Arc::new(image);
        self.modules.insert(name, arc.clone());
        arc
    }

    pub fn get(&self, name: &str) -> Option<Arc<BytecodeModule>> {
        self.modules.get(name).cloned()
    }
}

pub struct RuntimeLoader {
    registry: Arc<Mutex<ModuleRegistry>>,
}

impl RuntimeLoader {
    pub fn new(registry: Arc<Mutex<ModuleRegistry>>) -> Self {
        Self { registry }
    }

    pub fn load(&self, image: BytecodeModule) -> Arc<BytecodeModule> {
        self.registry.lock().unwrap().register(image)
    }
}

pub struct LogicalThread {
    id: usize,
    state: ThreadState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Running,
    Suspended,
    Terminated,
}

impl LogicalThread {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            state: ThreadState::Running,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn state(&self) -> ThreadState {
        self.state
    }
}

pub struct Runtime {
    registry: Arc<Mutex<ModuleRegistry>>,
    threads: Vec<LogicalThread>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(ModuleRegistry::new())),
            threads: Vec::new(),
        }
    }

    pub fn spawn_thread(&mut self) -> usize {
        let id = self.threads.len();
        self.threads.push(LogicalThread::new(id));
        id
    }

    pub fn threads(&self) -> &[LogicalThread] {
        &self.threads
    }

    pub fn registry(&self) -> Arc<Mutex<ModuleRegistry>> {
        self.registry.clone()
    }

    pub fn loader(&self) -> RuntimeLoader {
        RuntimeLoader::new(self.registry())
    }

    /// Execute an entry exported by a module loaded in the given BytecodeGraph.
    pub fn run_module_entry(
        &mut self,
        graph: &galfus_bytecode::BytecodeGraph,
        module_id: galfus_core::ModuleId,
        entry_name: &str,
        args: &[Vec<u8>],
        providers: Option<Providers>,
    ) -> Result<i32, RuntimeError> {
        let image = &graph.get(module_id).unwrap().image;
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

        let mut vm = VirtualMachine::new(graph).with_providers(providers);

        let result = (|| {
            if let Some(init_idx) = image.init_func_idx {
                vm.run_function(module_id, init_idx, vec![])?;
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
    let uint8_ty = find_type(&vm.graph.get(module_id).unwrap().image, |ty| {
        matches!(ty, galfus_bytecode::ImageType::Uint8)
    })
    .ok_or(RuntimeError::MissingArgumentType("u8"))?;
    let byte_array_ty = vm
        .graph.get(module_id).unwrap().image
        .types
        .iter()
        .enumerate()
        .find(|(_, ty)| {
            matches!(ty, galfus_bytecode::ImageType::Array(element)
                if matches!(vm.graph.get(module_id).unwrap().image.types.get(element.raw() as usize), Some(galfus_bytecode::ImageType::Uint8)))
        })
        .map(|(index, _)| galfus_bytecode::instruction::TypeIdx(index as u16))
        .ok_or(RuntimeError::MissingArgumentType("[u8]"))?;
    let args_array_ty = vm
        .graph.get(module_id).unwrap().image
        .types
        .iter()
        .enumerate()
        .find(|(_, ty)| {
            matches!(ty, galfus_bytecode::ImageType::Array(element)
                if matches!(vm.graph.get(module_id).unwrap().image.types.get(element.raw() as usize), Some(galfus_bytecode::ImageType::Array(inner))
                    if matches!(vm.graph.get(module_id).unwrap().image.types.get(inner.raw() as usize), Some(galfus_bytecode::ImageType::Uint8))))
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
    image: &BytecodeModule,
    predicate: impl Fn(&galfus_bytecode::ImageType) -> bool,
) -> Option<galfus_bytecode::instruction::TypeIdx> {
    image
        .types
        .iter()
        .position(predicate)
        .map(|index| galfus_bytecode::instruction::TypeIdx(index as u16))
}
