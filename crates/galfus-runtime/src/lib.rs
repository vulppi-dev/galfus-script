mod module_graph;

use galfus_compiler::CompiledModuleImage;
use galfus_core::ModuleId;
use galfus_image::ModuleImage;
use galfus_target::TargetCapabilityProvider;
use galfus_vm::{HeapObject, VirtualMachine, VmPanic, VmValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(test)]
mod tests;

pub use module_graph::{LinkedImport, ModuleLink, RuntimeLinkError, RuntimeModuleGraph};

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
    #[error("runtime target provider is unavailable")]
    TargetUnavailable,
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

    fn accepts_return_type(self, ty: &galfus_image::ImageType) -> bool {
        match self.return_type {
            EntryReturnType::Int32 => ty == &galfus_image::ImageType::Int32,
        }
    }
}

pub struct ModuleRegistry {
    modules: HashMap<String, Arc<ModuleImage>>,
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

    pub fn register(&mut self, image: ModuleImage) -> Arc<ModuleImage> {
        let name = image.name.clone();
        let arc = Arc::new(image);
        self.modules.insert(name, arc.clone());
        arc
    }

    pub fn get(&self, name: &str) -> Option<Arc<ModuleImage>> {
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

    pub fn load(&self, image: ModuleImage) -> Arc<ModuleImage> {
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
    modules: RuntimeModuleGraph,
    threads: Vec<LogicalThread>,
    capabilities: Option<Box<dyn TargetCapabilityProvider>>,
}

impl Runtime {
    pub fn new(capabilities: Box<dyn TargetCapabilityProvider>) -> Self {
        Self {
            registry: Arc::new(Mutex::new(ModuleRegistry::new())),
            modules: RuntimeModuleGraph::new(),
            threads: Vec::new(),
            capabilities: Some(capabilities),
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

    /// Upsert a compiled module using its stable `ModuleId`.
    pub fn load(&mut self, image: CompiledModuleImage) -> Option<CompiledModuleImage> {
        self.modules.load(image)
    }

    /// Remove a compiled module and its path lookup entry.
    pub fn unload(&mut self, id: ModuleId) -> Option<CompiledModuleImage> {
        self.modules.unload(id)
    }

    pub fn modules(&self) -> &RuntimeModuleGraph {
        &self.modules
    }

    /// Resolve a module's import slots against the currently loaded modules.
    pub fn link_module(&self, id: ModuleId) -> Result<ModuleLink, RuntimeLinkError> {
        self.modules.link(id)
    }

    pub fn initialization_order(&self, id: ModuleId) -> Result<Vec<ModuleId>, RuntimeLinkError> {
        self.modules.initialization_order(id)
    }

    pub fn run_entry(
        &mut self,
        module_name: &str,
        entry_name: &str,
        args: &[Vec<u8>],
    ) -> Result<i32, RuntimeError> {
        let abi = EntryAbi::default_app();
        let image = self
            .registry
            .lock()
            .unwrap()
            .get(module_name)
            .ok_or_else(|| RuntimeError::ModuleNotLoaded(module_name.to_string()))?;
        let image = (*image).clone();
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

        let target = self
            .capabilities
            .take()
            .ok_or(RuntimeError::TargetUnavailable)?;
        let mut vm = VirtualMachine::new(image).with_target(target);

        let result = (|| {
            if let Some(init_idx) = vm.image.init_func_idx {
                vm.run_function(init_idx, vec![])?;
            }

            let entry_args = build_entry_args(&mut vm, args)?;
            vm.run_function(entry_idx, vec![entry_args])
                .map_err(RuntimeError::VmPanic)
        })();
        self.capabilities = Some(vm.context.target);
        let result = result?;

        match result {
            VmValue::Int32(code) => Ok(code),
            _ => Err(RuntimeError::EntryReturnTypeMismatch {
                name: entry_name.to_string(),
            }),
        }
    }
}

fn build_entry_args(vm: &mut VirtualMachine, args: &[Vec<u8>]) -> Result<VmValue, RuntimeError> {
    let uint8_ty = find_type(&vm.image, |ty| matches!(ty, galfus_image::ImageType::Uint8))
        .ok_or(RuntimeError::MissingArgumentType("u8"))?;
    let byte_array_ty = find_type(
        &vm.image,
        |ty| matches!(ty, galfus_image::ImageType::Array(element) if *element == uint8_ty),
    )
    .ok_or(RuntimeError::MissingArgumentType("[u8]"))?;
    let args_array_ty = find_type(
        &vm.image,
        |ty| matches!(ty, galfus_image::ImageType::Array(element) if *element == byte_array_ty),
    )
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
    image: &ModuleImage,
    predicate: impl Fn(&galfus_image::ImageType) -> bool,
) -> Option<galfus_image::instruction::TypeIdx> {
    image
        .types
        .iter()
        .position(predicate)
        .map(|index| galfus_image::instruction::TypeIdx(index as u16))
}
