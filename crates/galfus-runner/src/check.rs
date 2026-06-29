use crate::*;

mod references;
use anyhow::Result;
use galfus_core::{Diagnostic, DiagnosticBag, NodeId, SourceFile, SourceId, SymbolId};
use galfus_frontend::{
    ImportKind, ImportedSurfaceTypes, ModuleGraph, ModuleSurface, ResolutionLayer, SyntaxNodeKind,
    TypeCheckResult, build_module_surface, check_declaration_types,
    check_declaration_types_with_surfaces, imported_surface_types_for_named_export,
    imported_surface_types_for_namespace, parse, resolve,
};
pub(crate) use references::check_path;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct CheckedModule {
    path: PathBuf,
    source: SourceFile,
    graph: ModuleGraph,
    type_result: Option<TypeCheckResult>,
}

impl CheckedModule {
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn source(&self) -> &SourceFile {
        &self.source
    }

    pub fn graph(&self) -> &ModuleGraph {
        &self.graph
    }

    pub fn type_result(&self) -> Option<&TypeCheckResult> {
        self.type_result.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub modules: Vec<CheckedModule>,
    pub diagnostics: DiagnosticBag,
}

impl CheckResult {
    pub fn modules(&self) -> &[CheckedModule] {
        self.modules.as_slice()
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        &self.diagnostics
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    pub fn source_for(&self, source_id: SourceId) -> Option<&SourceFile> {
        self.modules
            .iter()
            .find(|module| module.source().id() == source_id)
            .map(CheckedModule::source)
    }
}

#[derive(Debug, Clone)]
struct ImportCheckRecord {
    kind: ImportKind,
    source: String,
    local_name: String,
    imported_name: Option<String>,
    declaration: NodeId,
    local_symbol: SymbolId,
}

#[derive(Debug, Clone)]
struct PathSegmentRecord {
    name: String,
    node: NodeId,
}

#[derive(Debug, Clone)]
struct PathCheckRecord {
    node: NodeId,
    segments: Vec<PathSegmentRecord>,
}

#[derive(Debug, Clone)]
struct NamedTypeCheckRecord {
    node: NodeId,
    name: String,
}

#[derive(Debug, Default)]
pub(crate) struct ModuleLoader {
    pub(crate) modules: Vec<CheckedModule>,
    module_by_path: HashMap<PathBuf, usize>,
    loading: HashSet<PathBuf>,
    pub(crate) diagnostics: DiagnosticBag,
    resolver: WorkspaceResolver,
}

impl ModuleLoader {
    fn check_entry(&mut self, path: &Path) -> Result<()> {
        let path = normalize_existing_path(path)?;

        self.load_module(path)?;
        self.validate_imports();
        self.type_check_modules();

        Ok(())
    }

    pub(crate) fn load_module(&mut self, path: PathBuf) -> Result<usize> {
        let source = if path == Path::new(STD_IO_MODULE) {
            ModuleSource::Builtin {
                name: STD_IO_MODULE.to_string(),
            }
        } else {
            ModuleSource::File(path)
        };
        self.load_module_source(source)
    }

    fn load_module_source(&mut self, source: ModuleSource) -> Result<usize> {
        let path = source.path();
        if let Some(module) = self.module_by_path.get(path.as_path()).copied() {
            return Ok(module);
        }

        if self.loading.contains(path.as_path()) {
            return Ok(self.modules.len());
        }

        self.loading.insert(path.clone());

        let source_id = SourceId::new(self.modules.len() as u32);
        let text = self.resolver.read(&source)?;
        let source = SourceFile::new(source_id, path.display().to_string(), text);

        let parse_result = parse(&source);
        let resolve_result = resolve(&source, parse_result.into_graph());
        let graph = resolve_result.into_graph();

        self.diagnostics.extend(graph.diagnostics().iter().cloned());

        let module_index = self.modules.len();
        self.modules.push(CheckedModule {
            path: path.clone(),
            source,
            graph,
            type_result: None,
        });
        self.module_by_path.insert(path.clone(), module_index);

        self.load_relative_imports(module_index)?;

        self.loading.remove(path.as_path());

        Ok(module_index)
    }

    fn load_relative_imports(&mut self, module_index: usize) -> Result<()> {
        let imports = self.import_sources(module_index);

        for (source, source_node) in imports {
            if !is_resolvable_import(source.as_str()) {
                continue;
            }

            let module_source = match self
                .resolver
                .resolve_import(self.modules[module_index].path(), source.as_str())
            {
                Ok(module_source) => module_source,
                Err(_) => {
                    let span = self.modules[module_index]
                        .graph()
                        .syntax()
                        .node(source_node)
                        .map(|node| node.span())
                        .unwrap_or_else(|| self.modules[module_index].source().span());

                    self.diagnostics.push(Diagnostic::error_with_message(
                        CheckDiagnosticCode::ImportModuleNotFound,
                        format!("import module `{source}` not found"),
                        span,
                    ));
                    continue;
                }
            };
            let path = module_source.path();

            if path.extension().and_then(|extension| extension.to_str()) != Some("gfs") {
                if matches!(module_source, ModuleSource::Builtin { .. }) {
                    self.load_module_source(module_source)?;
                    continue;
                }

                let span = self.modules[module_index]
                    .graph()
                    .syntax()
                    .node(source_node)
                    .map(|node| node.span())
                    .unwrap_or_else(|| self.modules[module_index].source().span());

                self.diagnostics.push(Diagnostic::error_with_message(
                    CheckDiagnosticCode::UnsupportedImportTarget,
                    format!("import `{source}` must resolve to a .gfs source file"),
                    span,
                ));

                continue;
            }

            self.load_module_source(module_source)?;
        }

        Ok(())
    }

    fn import_sources(&self, module_index: usize) -> Vec<(String, galfus_core::NodeId)> {
        let Some(resolution) = self.modules[module_index].graph().resolution() else {
            return Vec::new();
        };

        resolution
            .imports()
            .iter()
            .map(|import| (import.source().to_string(), import.source_node()))
            .collect()
    }

    pub(crate) fn validate_imports(&mut self) {
        for module_index in 0..self.modules.len() {
            let imports = self.module_imports(module_index);

            for import in imports {
                if import.kind != ImportKind::Named || !is_resolvable_import(import.source.as_str())
                {
                    continue;
                }

                let Ok(source) = self
                    .resolver
                    .resolve_import(self.modules[module_index].path(), import.source.as_str())
                else {
                    continue;
                };
                let path = source.path();

                let Some(target_index) = self.module_by_path.get(path.as_path()).copied() else {
                    continue;
                };

                let Some(target_resolution) = self.modules[target_index].graph().resolution()
                else {
                    continue;
                };

                let Some(imported_name) = import.imported_name else {
                    continue;
                };

                if target_resolution
                    .export_by_name(imported_name.as_str())
                    .is_some()
                {
                    continue;
                }

                let span = self.modules[module_index]
                    .graph()
                    .syntax()
                    .node(import.declaration)
                    .map(|node| node.span())
                    .unwrap_or_else(|| self.modules[module_index].source().span());

                self.diagnostics.push(Diagnostic::error_with_message(
                    CheckDiagnosticCode::MissingExport,
                    format!(
                        "module `{}` does not export `{}`",
                        import.source, imported_name
                    ),
                    span,
                ));
            }
        }

        self.validate_namespace_import_references();
    }

    fn module_imports(&self, module_index: usize) -> Vec<ImportCheckRecord> {
        let Some(resolution) = self.modules[module_index].graph().resolution() else {
            return Vec::new();
        };

        resolution
            .imports()
            .iter()
            .map(|import| ImportCheckRecord {
                kind: import.kind(),
                source: import.source().to_string(),
                local_name: import.local_name().to_string(),
                imported_name: import.imported_name().map(str::to_string),
                declaration: import.declaration(),
                local_symbol: import.local_symbol(),
            })
            .collect()
    }

    pub(crate) fn type_check_modules(&mut self) {
        let baseline_results = self
            .modules
            .iter()
            .map(|module| check_declaration_types(module.source(), module.graph()))
            .collect::<Vec<_>>();

        let surfaces = self
            .modules
            .iter()
            .zip(baseline_results.iter())
            .map(|(module, result)| build_module_surface(module.graph(), result))
            .collect::<Vec<_>>();

        let imported_types = (0..self.modules.len())
            .map(|module_index| self.imported_surface_types_for_module(module_index, &surfaces))
            .collect::<Vec<_>>();

        for (module_index, imported_type) in imported_types.iter().enumerate() {
            let result = check_declaration_types_with_surfaces(
                self.modules[module_index].source(),
                self.modules[module_index].graph(),
                imported_type,
            );

            self.diagnostics
                .extend(result.diagnostics().iter().cloned());
            self.modules[module_index].type_result = Some(result);
        }
    }

    fn imported_surface_types_for_module(
        &self,
        module_index: usize,
        surfaces: &[ModuleSurface],
    ) -> ImportedSurfaceTypes {
        let mut imported_types = ImportedSurfaceTypes::new();

        for import in self.module_imports(module_index) {
            if import.kind != ImportKind::Named || !is_resolvable_import(import.source.as_str()) {
                continue;
            }

            let Some(target_index) = self.import_target_index(module_index, import.source.as_str())
            else {
                continue;
            };

            let Some(imported_name) = import.imported_name.as_deref() else {
                continue;
            };

            let Some(imported_type) =
                surfaces[target_index].imported_type_for_export(import.local_symbol, imported_name)
            else {
                if let Some(imported_constraint) =
                    surfaces[target_index].imported_constraint_for_export(imported_name)
                {
                    imported_types
                        .insert_symbol_constraint(import.local_symbol, imported_constraint);
                }

                continue;
            };

            imported_types.insert_symbol_type(import.local_symbol, imported_type);

            if let Some(imported_constraint) =
                surfaces[target_index].imported_constraint_for_export(imported_name)
            {
                imported_types.insert_symbol_constraint(import.local_symbol, imported_constraint);
            }

            if let Some(imported_choice) =
                surfaces[target_index].imported_choice_for_export(imported_name)
            {
                imported_types.insert_symbol_choice(import.local_symbol, imported_choice);
            }

            imported_types.extend(imported_surface_types_for_named_export(
                &surfaces[target_index],
                import.local_symbol,
                imported_name,
            ));
        }

        self.collect_named_imported_type_types(module_index, surfaces, &mut imported_types);
        self.collect_named_imported_path_types(module_index, surfaces, &mut imported_types);
        self.collect_namespace_imported_path_types(module_index, surfaces, &mut imported_types);

        imported_types
    }

    fn collect_named_imported_type_types(
        &self,
        module_index: usize,
        surfaces: &[ModuleSurface],
        imported_types: &mut ImportedSurfaceTypes,
    ) {
        let imports = self.module_imports(module_index);
        let named_imports = imports
            .iter()
            .filter(|import| {
                import.kind == ImportKind::Named && is_resolvable_import(import.source.as_str())
            })
            .collect::<Vec<_>>();

        if named_imports.is_empty() {
            return;
        }

        for named_type in self.named_type_records(module_index) {
            let Some(import) = named_imports
                .iter()
                .find(|import| import.local_name == named_type.name)
            else {
                continue;
            };

            let Some(imported_name) = import.imported_name.as_deref() else {
                continue;
            };

            let Some(target_index) = self.import_target_index(module_index, import.source.as_str())
            else {
                continue;
            };

            let Some(imported_type) =
                surfaces[target_index].imported_type_for_export(import.local_symbol, imported_name)
            else {
                continue;
            };

            imported_types.insert_path_type(named_type.node, imported_type);
        }
    }

    fn import_target_index(&self, module_index: usize, source: &str) -> Option<usize> {
        let module_source = self
            .resolver
            .resolve_import(self.modules[module_index].path(), source)
            .ok()?;
        self.module_by_path
            .get(module_source.path().as_path())
            .copied()
    }
}

pub fn check_file(path: &str) -> Result<()> {
    let result = check_path(path)?;
    print_check_result(&result);
    Ok(())
}

fn is_relative_import(source: &str) -> bool {
    source.starts_with("./") || source.starts_with("../")
}

fn is_builtin_import(source: &str) -> bool {
    source == STD_IO_MODULE
}

fn is_resolvable_import(source: &str) -> bool {
    is_relative_import(source) || is_builtin_import(source)
}
