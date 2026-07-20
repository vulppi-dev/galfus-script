use galfus_bytecode::graph::CompiledBytecodeModule;
use galfus_bytecode::{
    BytecodeModule, Constant, ConstantPool, ExportSlot, ImageFunction, ImageType,
    instruction::{ChoiceLayoutIdx, ConstIdx, FuncIdx, Instruction, Reg, StructLayoutIdx, TypeIdx},
};
use galfus_core::{ModuleId, ModulePath};
use std::collections::{HashMap, HashSet};

/// A resolved runtime target for one import slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedImport {
    pub slot: usize,
    pub module_id: ModuleId,
    pub function: FuncIdx,
}

/// The dynamic linking result for one loaded module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleLink {
    pub module_id: ModuleId,
    pub imports: Vec<LinkedImport>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeLinkError {
    #[error("module {0:?} is not loaded")]
    ModuleNotLoaded(ModuleId),
    #[error("module {importer:?} imports unloaded module `{module_path}`")]
    ImportModuleNotLoaded {
        importer: ModuleId,
        module_path: String,
    },
    #[error("module {importer:?} imports missing function `{symbol_name}` from `{module_path}`")]
    ImportSymbolNotExported {
        importer: ModuleId,
        module_path: String,
        symbol_name: String,
    },
    #[error("module initialization cycle includes {0:?}")]
    InitializationCycle(ModuleId),
}

/// Loaded compiled modules, indexed by their stable IDs.
#[derive(Debug, Default)]
pub struct RuntimeModuleGraph {
    modules: HashMap<ModuleId, CompiledBytecodeModule>,
    ids_by_path: HashMap<ModulePath, ModuleId>,
}

impl RuntimeModuleGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a module. Existing import links are resolved lazily.
    pub fn load(&mut self, image: CompiledBytecodeModule) -> Option<CompiledBytecodeModule> {
        if let Some(previous) = self.modules.get(&image.id)
            && previous.path != image.path
        {
            self.ids_by_path.remove(&previous.path);
        }
        self.ids_by_path.insert(image.path.clone(), image.id);
        self.modules.insert(image.id, image)
    }

    pub fn unload(&mut self, id: ModuleId) -> Option<CompiledBytecodeModule> {
        let image = self.modules.remove(&id)?;
        self.ids_by_path.remove(&image.path);
        Some(image)
    }

    pub fn get(&self, id: ModuleId) -> Option<&CompiledBytecodeModule> {
        self.modules.get(&id)
    }

    /// Stable IDs for every module currently loaded by the runtime.
    pub fn module_ids(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.modules.keys().copied()
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Resolve each import slot to an exported function in another loaded module.
    pub fn link(&self, id: ModuleId) -> Result<ModuleLink, RuntimeLinkError> {
        let image = self
            .modules
            .get(&id)
            .ok_or(RuntimeLinkError::ModuleNotLoaded(id))?;
        let mut imports = Vec::with_capacity(image.image.imports.len());

        for (slot, import) in image.image.imports.iter().enumerate() {
            let module_id = ModulePath::new(import.module_name.as_str())
                .and_then(|path| self.ids_by_path.get(&path).copied())
                .ok_or_else(|| RuntimeLinkError::ImportModuleNotLoaded {
                    importer: id,
                    module_path: import.module_name.clone(),
                })?;
            let target = self
                .modules
                .get(&module_id)
                .expect("path index refers to loaded module");
            let function = target
                .image
                .exports
                .iter()
                .find(|export| export.symbol_name == import.symbol_name)
                .map(|export| export.func_idx)
                .ok_or_else(|| RuntimeLinkError::ImportSymbolNotExported {
                    importer: id,
                    module_path: import.module_name.clone(),
                    symbol_name: import.symbol_name.clone(),
                })?;
            imports.push(LinkedImport {
                slot,
                module_id,
                function,
            });
        }

        Ok(ModuleLink {
            module_id: id,
            imports,
        })
    }

    /// Return modules in dependency-first initialization order for `id`.
    pub fn initialization_order(&self, id: ModuleId) -> Result<Vec<ModuleId>, RuntimeLinkError> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        self.collect_initialization_order(id, &mut order, &mut visited, &mut visiting)?;
        Ok(order)
    }

    /// Link the reachable module subgraph into one VM image for execution.
    /// Module images remain separately stored; this image is an execution view.
    pub fn linked_image(&self, entry: ModuleId) -> Result<BytecodeModule, RuntimeLinkError> {
        let order = self.initialization_order(entry)?;
        let mut function_bases = HashMap::new();
        let mut constant_bases = HashMap::new();
        let mut type_bases = HashMap::new();
        let mut struct_bases = HashMap::new();
        let mut choice_bases = HashMap::new();
        let mut function_count = 0u16;
        let mut constant_count = 0u16;
        let mut type_count = 0u16;
        let mut struct_count = 0u16;
        let mut choice_count = 0u16;
        for id in &order {
            let image = &self.modules[id].image;
            function_bases.insert(*id, function_count);
            constant_bases.insert(*id, constant_count);
            type_bases.insert(*id, type_count);
            struct_bases.insert(*id, struct_count);
            choice_bases.insert(*id, choice_count);
            function_count += image.functions.len() as u16;
            constant_count += image.constants.constants.len() as u16;
            type_count += image.types.len() as u16;
            struct_count += image.struct_layouts.len() as u16;
            choice_count += image.choice_layouts.len() as u16;
        }

        let mut functions = Vec::new();
        let mut constants = Vec::new();
        let mut types = Vec::new();
        let mut struct_layouts = Vec::new();
        let mut choice_layouts = Vec::new();
        let mut init_calls = Vec::new();
        for id in &order {
            let module = &self.modules[id];
            let image = &module.image;
            let link = self.link(*id)?;
            let function_base = function_bases[id];
            let constant_base = constant_bases[id];
            let type_base = type_bases[id];
            let struct_base = struct_bases[id];
            let choice_base = choice_bases[id];
            let import_targets = link
                .imports
                .iter()
                .map(|import| FuncIdx(function_bases[&import.module_id] + import.function.raw()))
                .collect::<Vec<_>>();

            for constant in &image.constants.constants {
                constants.push(match constant {
                    Constant::Function(index) if (index.raw() as usize) < image.functions.len() => {
                        Constant::Function(FuncIdx(function_base + index.raw()))
                    }
                    constant => constant.clone(),
                });
            }
            for ty in &image.types {
                types.push(relocate_type(
                    ty.clone(),
                    type_base,
                    struct_base,
                    choice_base,
                ));
            }
            for layout in &image.struct_layouts {
                let mut layout = layout.clone();
                for field in &mut layout.fields {
                    field.ty = TypeIdx(type_base + field.ty.raw());
                }
                struct_layouts.push(layout);
            }
            for layout in &image.choice_layouts {
                let mut layout = layout.clone();
                for variant in &mut layout.variants {
                    if let Some(payload) = &mut variant.payload_ty {
                        *payload = TypeIdx(type_base + payload.raw());
                    }
                }
                choice_layouts.push(layout);
            }
            for function in &image.functions {
                let mut function = function.clone();
                function.return_ty = TypeIdx(type_base + function.return_ty.raw());
                for instruction in &mut function.instructions {
                    relocate_instruction(
                        instruction,
                        image.functions.len(),
                        function_base,
                        constant_base,
                        type_base,
                        import_targets.as_slice(),
                    );
                }
                functions.push(function);
            }
            if let Some(init) = image.init_func_idx {
                init_calls.push(FuncIdx(function_base + init.raw()));
            }
        }

        let entry_image = &self.modules[&entry].image;
        let entry_base = function_bases[&entry];
        let exports = entry_image
            .exports
            .iter()
            .map(|export| ExportSlot {
                symbol_name: export.symbol_name.clone(),
                func_idx: FuncIdx(entry_base + export.func_idx.raw()),
            })
            .collect();
        let init_func_idx = if init_calls.is_empty() {
            None
        } else {
            let null_type = types
                .iter()
                .position(|ty| matches!(ty, ImageType::Null))
                .map(|index| TypeIdx(index as u16))
                .unwrap_or_else(|| {
                    let index = TypeIdx(types.len() as u16);
                    types.push(ImageType::Null);
                    index
                });
            let index = FuncIdx(functions.len() as u16);
            functions.push(ImageFunction {
                name: "__init_runtime_graph".to_string(),
                param_count: 0,
                local_count: 0,
                temp_count: 1,
                return_ty: null_type,
                instructions: init_calls
                    .into_iter()
                    .map(|func| Instruction::Call {
                        dest: Reg(0),
                        func,
                        args_start: Reg(0),
                        arg_count: 0,
                    })
                    .chain(std::iter::once(Instruction::RetNull))
                    .collect(),
            });
            Some(index)
        };

        Ok(BytecodeModule {
            name: entry_image.name.clone(),
            constants: ConstantPool { constants },
            functions,
            types,
            struct_layouts,
            choice_layouts,
            imports: Vec::new(),
            exports,
            init_func_idx,
        })
    }

    fn collect_initialization_order(
        &self,
        id: ModuleId,
        order: &mut Vec<ModuleId>,
        visited: &mut HashSet<ModuleId>,
        visiting: &mut HashSet<ModuleId>,
    ) -> Result<(), RuntimeLinkError> {
        if visited.contains(&id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            return Err(RuntimeLinkError::InitializationCycle(id));
        }

        for import in self.link(id)?.imports {
            self.collect_initialization_order(import.module_id, order, visited, visiting)?;
        }

        visiting.remove(&id);
        visited.insert(id);
        order.push(id);
        Ok(())
    }
}

fn relocate_instruction(
    instruction: &mut Instruction,
    own_function_count: usize,
    function_base: u16,
    constant_base: u16,
    type_base: u16,
    import_targets: &[FuncIdx],
) {
    match instruction {
        Instruction::Call { func, .. } => {
            let index = func.raw() as usize;
            *func = if index < own_function_count {
                FuncIdx(function_base + func.raw())
            } else {
                import_targets[index - own_function_count]
            };
        }
        Instruction::LoadConst { const_idx, .. }
        | Instruction::Panic { const_idx }
        | Instruction::CallMethod {
            name_const: const_idx,
            ..
        } => {
            *const_idx = ConstIdx(constant_base + const_idx.raw());
        }
        Instruction::AllocLocal { type_idx, .. }
        | Instruction::AllocShared { type_idx, .. }
        | Instruction::NewArray { type_idx, .. }
        | Instruction::NewTuple { type_idx, .. }
        | Instruction::NewChoice { type_idx, .. }
        | Instruction::Cast { type_idx, .. }
        | Instruction::Instanceof { type_idx, .. } => {
            *type_idx = TypeIdx(type_base + type_idx.raw());
        }
        _ => {}
    }
}

fn relocate_type(ty: ImageType, type_base: u16, struct_base: u16, choice_base: u16) -> ImageType {
    match ty {
        ImageType::Struct(index) => ImageType::Struct(StructLayoutIdx(struct_base + index.raw())),
        ImageType::Array(index) => ImageType::Array(TypeIdx(type_base + index.raw())),
        ImageType::Tuple(indices) => ImageType::Tuple(
            indices
                .into_iter()
                .map(|index| TypeIdx(type_base + index.raw()))
                .collect(),
        ),
        ImageType::Choice(index) => ImageType::Choice(ChoiceLayoutIdx(choice_base + index.raw())),
        ImageType::Function { params, ret } => ImageType::Function {
            params: params
                .into_iter()
                .map(|index| TypeIdx(type_base + index.raw()))
                .collect(),
            ret: TypeIdx(type_base + ret.raw()),
        },
        ImageType::ChoiceVariant(index, variant) => {
            ImageType::ChoiceVariant(ChoiceLayoutIdx(choice_base + index.raw()), variant)
        }
        ty => ty,
    }
}
