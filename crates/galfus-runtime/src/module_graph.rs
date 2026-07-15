use galfus_compiler::CompiledModuleImage;
use galfus_core::{ModuleId, ModulePath};
use galfus_image::instruction::FuncIdx;
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
    modules: HashMap<ModuleId, CompiledModuleImage>,
    ids_by_path: HashMap<ModulePath, ModuleId>,
}

impl RuntimeModuleGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a module. Existing import links are resolved lazily.
    pub fn load(&mut self, image: CompiledModuleImage) -> Option<CompiledModuleImage> {
        if let Some(previous) = self.modules.get(&image.id)
            && previous.path != image.path
        {
            self.ids_by_path.remove(&previous.path);
        }
        self.ids_by_path.insert(image.path.clone(), image.id);
        self.modules.insert(image.id, image)
    }

    pub fn unload(&mut self, id: ModuleId) -> Option<CompiledModuleImage> {
        let image = self.modules.remove(&id)?;
        self.ids_by_path.remove(&image.path);
        Some(image)
    }

    pub fn get(&self, id: ModuleId) -> Option<&CompiledModuleImage> {
        self.modules.get(&id)
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
