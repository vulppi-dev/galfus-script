use crate::ImportKind;
use crate::diagnostics::CheckDiagnosticCode;
use crate::modules::graph::SemanticModuleGraph;
use crate::modules::module::SemanticModule;
use crate::modules::resolution::{is_resolvable_import, resolve_relative_import};
use crate::{
    ImportedSurfaceTypes, ModuleGraph, ModuleSurface, TypeCheckResult, build_module_surface,
    check_declaration_types, check_definition_types_with_surfaces,
    imported_surface_types_for_named_export, parse, resolve,
};
use galfus_core::{
    Diagnostic, DiagnosticBag, ModuleId, ModulePath, NodeId, Revision, SourceFile, SourceId,
    SymbolId,
};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub(crate) struct ImportCheckRecord {
    pub(crate) kind: ImportKind,
    pub(crate) source: String,
    pub(crate) local_name: String,
    pub(crate) imported_name: Option<String>,
    pub(crate) declaration: NodeId,
    pub(crate) local_symbol: SymbolId,
}

#[derive(Debug, Clone)]
pub(crate) struct NamedTypeCheckRecord {
    pub(crate) node: NodeId,
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PathSegmentRecord {
    pub(crate) name: String,
    pub(crate) node: NodeId,
}

#[derive(Debug, Clone)]
pub(crate) struct PathCheckRecord {
    pub(crate) node: NodeId,
    pub(crate) segments: Vec<PathSegmentRecord>,
}

#[derive(Default)]
pub struct FrontendRoots {
    roots: Vec<crate::modules::graph::SemanticRoot>,
}

impl FrontendRoots {
    pub fn new(roots: Vec<crate::modules::graph::SemanticRoot>) -> Self {
        Self { roots }
    }

    pub fn roots(&self) -> &[crate::modules::graph::SemanticRoot] {
        self.roots.as_slice()
    }
}

pub struct FrontendSource<'a> {
    pub module_id: ModuleId,
    pub path: ModulePath,
    pub source: &'a SourceFile,
}

pub struct FrontendUpdate<'a> {
    pub source_revision: Revision,
    pub sources: &'a [FrontendSource<'a>],
    pub roots: &'a FrontendRoots,
}

pub struct FrontendReport {
    pub source_revision: Revision,
    pub semantic_revision: galfus_core::SemanticRevision,
    pub diagnostics: DiagnosticBag,
}

#[derive(Default)]
pub struct FrontendSession {
    pub modules: Vec<SemanticModule>,
    module_by_path: HashMap<ModulePath, usize>,
    semantic_graph: SemanticModuleGraph,
    pub diagnostics: DiagnosticBag,
    /// Global counter. Incremented each time any module's semantic result changes.
    next_semantic_revision: u64,
}

impl FrontendSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&mut self, update: FrontendUpdate<'_>) -> FrontendReport {
        self.modules.clear();
        self.module_by_path.clear();
        self.diagnostics = DiagnosticBag::new();

        for input in update.sources {
            let parse_result = parse(input.source);
            let resolve_result = resolve(input.source, parse_result.into_graph());
            let graph = resolve_result.into_graph();

            self.diagnostics.extend(graph.diagnostics().iter().cloned());

            // Each module gets its own semantic_revision so the compiler can
            // detect which modules actually changed after a re-check.
            self.next_semantic_revision += 1;
            let semantic_revision = galfus_core::SemanticRevision::new(self.next_semantic_revision);

            self.modules.push(SemanticModule {
                id: input.module_id,
                source_id: input.source.id(),
                path: input.path.clone(),
                source_revision: update.source_revision,
                semantic_revision,
                source: input.source.clone(),
                graph,
                type_result: None,
            });
            self.module_by_path
                .insert(input.path.clone(), self.modules.len() - 1);
        }

        self.validate_imports();
        self.type_check_modules();
        self.semantic_graph = SemanticModuleGraph::build(update.roots.roots(), &self.modules);

        // Report the highest semantic revision produced in this check cycle.
        let semantic_revision = self
            .modules
            .iter()
            .map(|m| m.semantic_revision)
            .max()
            .unwrap_or(galfus_core::SemanticRevision::new(
                self.next_semantic_revision,
            ));

        FrontendReport {
            source_revision: update.source_revision,
            semantic_revision,
            diagnostics: self.diagnostics.clone(),
        }
    }

    pub fn semantic_graph(&self) -> &SemanticModuleGraph {
        &self.semantic_graph
    }

    fn validate_imports(&mut self) {
        for module_index in 0..self.modules.len() {
            let imports = self.module_imports(module_index);

            for import in imports {
                if import.kind != ImportKind::Named || !is_resolvable_import(import.source.as_str())
                {
                    continue;
                }

                let Some(target_path) = resolve_relative_import(
                    self.modules[module_index].path(),
                    import.source.as_str(),
                ) else {
                    continue;
                };

                let Some(target_index) = self.module_by_path.get(&target_path).copied() else {
                    let span = self.modules[module_index]
                        .graph()
                        .syntax()
                        .node(import.declaration)
                        .map(|node| node.span())
                        .unwrap_or_else(|| self.modules[module_index].source().span());

                    self.diagnostics.push(Diagnostic::error_with_message(
                        CheckDiagnosticCode::ImportModuleNotFound,
                        format!("import module `{}` not found", import.source),
                        span,
                    ));
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
    }

    pub(crate) fn module_imports(&self, module_index: usize) -> Vec<ImportCheckRecord> {
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

    fn type_check_modules(&mut self) {
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

        for ((module_index, imported_type), previous_result) in imported_types
            .iter()
            .enumerate()
            .zip(baseline_results.into_iter())
        {
            let result = check_definition_types_with_surfaces(
                self.modules[module_index].source(),
                self.modules[module_index].graph(),
                previous_result,
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

    pub(super) fn import_target_index(&self, module_index: usize, source: &str) -> Option<usize> {
        let module_path = self.modules[module_index].path();
        let target_path = resolve_relative_import(module_path, source)?;
        self.module_by_path.get(&target_path).copied()
    }
}
