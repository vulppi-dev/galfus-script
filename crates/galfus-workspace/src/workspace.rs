use crate::config::{WorkspaceConfig, parse_workspace_config};
use crate::source_store::{ModuleOrigin, SourceStore};
use crate::state::{CheckState, CompileBlocked, CompileState, RunBlocked, WorkspaceError};
use galfus_compiler::{CompiledImportEdge, CompiledModule, CompiledModuleGraph};
use galfus_core::{DiagnosticBag, ModuleId, ModulePath, Revision, SourceFile};
use galfus_frontend::modules::{
    FrontendRoots, FrontendSession, FrontendSource, FrontendUpdate, SemanticRoot, SemanticRootKind,
};
use galfus_runtime::Runtime;
use galfus_target::{NativeTarget, TargetCapabilityProvider};
use std::collections::HashSet;
use std::sync::Arc;

#[cfg(test)]
mod tests;

pub struct Workspace {
    sources: SourceStore,
    config: Option<WorkspaceConfig>,
    revision: Revision,
    check_state: CheckState,
    compile_state: CompileState,
    frontend: FrontendSession,
    runtime: Runtime,
    dirty_sources: HashSet<ModulePath>,
    removed_modules: Vec<ModuleId>,
}

pub enum LoadResult {
    Success,
    Diagnostics(DiagnosticBag),
}

pub enum RemoveResult {
    Success,
    NotFound,
}

pub struct CheckReport<'a> {
    pub is_valid: bool,
    pub diagnostics: &'a DiagnosticBag,
}

/// Result of a successful `compile()` call.
pub struct CompileReport {
    /// The compiled module graph, ready to be passed to the runtime.
    pub graph: Arc<CompiledModuleGraph>,
}

pub struct RunReport {
    pub exit_code: i32,
}

impl Workspace {
    pub fn new() -> Self {
        Self::with_target(Box::new(NativeTarget))
    }

    pub fn with_target(capabilities: Box<dyn TargetCapabilityProvider>) -> Self {
        Self {
            sources: SourceStore::new(),
            config: None,
            revision: Revision::new(1),
            check_state: CheckState::Dirty {
                current_revision: Revision::new(1),
                previous_checked_revision: None,
            },
            compile_state: CompileState::Missing,
            frontend: FrontendSession::new(),
            runtime: Runtime::new(capabilities),
            dirty_sources: HashSet::new(),
            removed_modules: Vec::new(),
        }
    }

    pub fn load_config(&mut self, config_toml: &[u8]) -> Result<LoadResult, WorkspaceError> {
        let text = match std::str::from_utf8(config_toml) {
            Ok(t) => t,
            Err(_) => return Err(WorkspaceError::MissingConfiguration),
        };

        let mut diagnostics = DiagnosticBag::new();
        if let Some(config) = parse_workspace_config(text, &mut diagnostics) {
            self.config = Some(config);
            self.mark_dirty();
            Ok(LoadResult::Success)
        } else {
            Ok(LoadResult::Diagnostics(diagnostics))
        }
    }

    pub fn load_module(
        &mut self,
        path: &str,
        module_bytes: &[u8],
    ) -> Result<LoadResult, WorkspaceError> {
        let module_path = ModulePath::new(path).ok_or(WorkspaceError::InvalidPath)?;
        if self
            .sources
            .get(&module_path)
            .is_some_and(|entry| entry.bytes.as_ref() == module_bytes)
        {
            return Ok(LoadResult::Success);
        }

        self.revision.next();
        self.sources.load_module(
            module_path.clone(),
            Arc::from(module_bytes),
            ModuleOrigin::User,
            self.revision,
        );
        self.dirty_sources.insert(module_path);
        self.mark_dirty();
        Ok(LoadResult::Success)
    }

    pub fn remove_module(&mut self, path: &str) -> Result<RemoveResult, WorkspaceError> {
        let module_path = ModulePath::new(path).ok_or(WorkspaceError::InvalidPath)?;

        if let Some(entry) = self.sources.remove_module(&module_path) {
            self.revision.next();
            self.dirty_sources.remove(&module_path);
            self.removed_modules.push(entry.module_id);
            self.mark_dirty();
            Ok(RemoveResult::Success)
        } else {
            Ok(RemoveResult::NotFound)
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.check_state.is_dirty()
    }

    fn mark_dirty(&mut self) {
        let previous = match &self.check_state {
            CheckState::Passed { revision, .. } | CheckState::Failed { revision, .. } => {
                Some(*revision)
            }
            CheckState::Dirty {
                previous_checked_revision,
                ..
            } => *previous_checked_revision,
        };

        self.check_state = CheckState::Dirty {
            current_revision: self.revision,
            previous_checked_revision: previous,
        };

        // Mark compile stale when check is invalidated.
        if let CompileState::Ready {
            semantic_revision,
            graph,
        } = &self.compile_state
        {
            self.compile_state = CompileState::Stale {
                semantic_revision: *semantic_revision,
                graph: Arc::clone(graph),
            };
        }
    }

    pub fn check(&mut self) -> CheckReport<'_> {
        let is_dirty = matches!(self.check_state, CheckState::Dirty { .. });

        if is_dirty {
            if self.config.is_none() {
                self.check_state = CheckState::Failed {
                    revision: self.revision,
                    diagnostics: DiagnosticBag::new(),
                };
            } else {
                let roots = self.frontend_roots();
                let report = loop {
                    let source_files = self
                        .dirty_sources
                        .iter()
                        .filter_map(|path| self.sources.get(path))
                        .map(|entry| {
                            (
                                entry.module_id,
                                entry.path.clone(),
                                SourceFile::new(
                                    entry.source_id,
                                    entry.path.to_string(),
                                    std::str::from_utf8(&entry.bytes).unwrap_or("").to_string(),
                                ),
                            )
                        })
                        .collect::<Vec<_>>();
                    let sources = source_files
                        .iter()
                        .map(|(module_id, path, source)| FrontendSource {
                            module_id: *module_id,
                            path: path.clone(),
                            source,
                        })
                        .collect::<Vec<_>>();
                    let update = FrontendUpdate {
                        source_revision: self.revision,
                        sources: &sources,
                        removed_modules: self.removed_modules.as_slice(),
                        roots: &roots,
                    };
                    let report = self.frontend.check(update);
                    self.dirty_sources.clear();
                    self.removed_modules.clear();

                    if !self.load_required_builtins(&report.required_builtin_modules) {
                        break report;
                    }
                };

                if report.diagnostics.has_errors() {
                    self.check_state = CheckState::Failed {
                        revision: report.source_revision,
                        diagnostics: report.diagnostics,
                    };
                } else {
                    self.check_state = CheckState::Passed {
                        revision: report.source_revision,
                        semantic_revision: report.semantic_revision,
                        changed_modules: report.changed_modules,
                        diagnostics: report.diagnostics,
                    };
                }
            }
        }

        match &self.check_state {
            CheckState::Passed { diagnostics, .. } => CheckReport {
                is_valid: true,
                diagnostics,
            },
            CheckState::Failed { diagnostics, .. } => CheckReport {
                is_valid: false,
                diagnostics,
            },
            CheckState::Dirty { .. } => unreachable!(),
        }
    }

    fn load_required_builtins(&mut self, paths: &HashSet<ModulePath>) -> bool {
        let mut loaded = false;
        for path in paths {
            if self.sources.get(path).is_some() {
                continue;
            }
            let builtin_name = path.as_str().strip_suffix(".gfs").unwrap_or(path.as_str());
            let Some((_, source)) = galfus_builtins::BUILTIN_MODULES
                .iter()
                .find(|(name, _)| *name == builtin_name)
            else {
                continue;
            };
            self.revision.next();
            self.sources.load_module(
                path.clone(),
                Arc::from(source.as_bytes()),
                ModuleOrigin::Builtin,
                self.revision,
            );
            self.dirty_sources.insert(path.clone());
            loaded = true;
        }
        loaded
    }

    fn frontend_roots(&self) -> FrontendRoots {
        let Some(config) = &self.config else {
            return FrontendRoots::default();
        };

        let mut roots = Vec::new();
        if let Some(entry) = config.entry() {
            if let Some(source) = self.sources.get(entry) {
                roots.push(SemanticRoot::new(
                    SemanticRootKind::Entry,
                    source.module_id,
                    entry.clone(),
                ));
            }
        }
        for export in config.exports() {
            if let Some(source) = self.sources.get(export.path()) {
                roots.push(SemanticRoot::new(
                    SemanticRootKind::Export {
                        address: export.address().to_string(),
                    },
                    source.module_id,
                    export.path().clone(),
                ));
            }
        }

        FrontendRoots::new(roots)
    }

    /// Compile the workspace into a [`CompiledModuleGraph`].
    ///
    /// Gate rules:
    /// - Returns `Err(CompileBlocked::Dirty)` if `check()` has not been called
    ///   since the last source change.
    /// - Returns `Err(CompileBlocked::CheckFailed)` if the last `check()` had errors.
    /// - Returns `Err(CompileBlocked::MissingConfiguration)` if no config was loaded.
    /// - Returns `Ok(CompileReport)` with the compiled graph on success.
    pub fn compile(&mut self) -> Result<CompileReport, CompileBlocked> {
        // Gate: check must have passed.
        let (semantic_revision, changed_modules) = match &self.check_state {
            CheckState::Dirty {
                current_revision,
                previous_checked_revision,
            } => {
                return Err(CompileBlocked::Dirty {
                    current_revision: *current_revision,
                    checked_revision: *previous_checked_revision,
                });
            }
            CheckState::Failed {
                revision,
                diagnostics,
            } => {
                return Err(CompileBlocked::CheckFailed {
                    revision: *revision,
                    error_count: diagnostics.iter().filter(|d| d.is_error()).count(),
                });
            }
            CheckState::Passed {
                semantic_revision,
                changed_modules,
                ..
            } => (*semantic_revision, changed_modules.clone()),
        };

        // Skip recompilation if already up-to-date.
        if let CompileState::Ready {
            semantic_revision: compiled_rev,
            graph,
        } = &self.compile_state
        {
            if *compiled_rev == semantic_revision {
                return Ok(CompileReport {
                    graph: Arc::clone(graph),
                });
            }
        }

        let cached_graph = match &self.compile_state {
            CompileState::Stale { graph, .. } => Some(graph),
            _ => None,
        };

        // The first compilation has no graph to upsert into, so it must emit
        // every semantic module even if the last frontend delta was narrower.
        let compilation_targets = if let Some(cached_graph) = cached_graph {
            self.frontend
                .modules
                .iter()
                .filter(|module| changed_modules.contains(&module.id()))
                .filter(|module| {
                    cached_graph
                        .get(module.id())
                        .is_none_or(|image| image.semantic_revision() != module.semantic_revision())
                })
                .map(|module| module.id())
                .collect::<HashSet<_>>()
        } else {
            self.frontend
                .modules
                .iter()
                .map(|module| module.id())
                .collect::<HashSet<_>>()
        };

        // Build CompiledModule list from the frontend's semantic modules.
        let semantic_modules = &self.frontend.modules;
        let mut compiled_modules: Vec<CompiledModule> = semantic_modules
            .iter()
            .map(|m| {
                CompiledModule::new(
                    m.id(),
                    m.path().clone(),
                    m.semantic_revision(),
                    m.source().clone(),
                    m.graph().clone(),
                    m.type_result().cloned(),
                )
            })
            .collect();

        // Compile each module individually — one ModuleImage per module.
        let outputs =
            galfus_compiler::compile_changed_modules(&mut compiled_modules, &compilation_targets)
                .map_err(|e| CompileBlocked::CompilerError(e.to_string()))?;

        // Build import edges from the SemanticModuleGraph.
        let semantic_graph = self.frontend.semantic_graph();
        let edges: Vec<CompiledImportEdge> = semantic_graph
            .import_edges()
            .iter()
            .filter_map(|edge| {
                let to = edge.to()?;
                Some(CompiledImportEdge {
                    from: edge.from(),
                    to,
                })
            })
            .collect();

        // Populate the CompiledModuleGraph — one image per module.
        let mut module_graph = cached_graph
            .map(|graph| (**graph).clone())
            .unwrap_or_else(CompiledModuleGraph::new);
        let current_modules = semantic_modules
            .iter()
            .map(|module| module.id())
            .collect::<HashSet<_>>();
        for id in &changed_modules {
            if !current_modules.contains(id) {
                module_graph.remove(*id);
            }
        }
        for image in outputs {
            module_graph.upsert(image);
        }
        module_graph.set_edges(edges);

        let graph = Arc::new(module_graph);
        self.compile_state = CompileState::Ready {
            semantic_revision,
            graph: Arc::clone(&graph),
        };

        Ok(CompileReport { graph })
    }

    /// Load the current compiled graph into the runtime and execute its configured entry.
    pub fn run(&mut self, args: &[Vec<u8>]) -> Result<RunReport, RunBlocked> {
        let graph = match &self.compile_state {
            CompileState::Ready { graph, .. } => Arc::clone(graph),
            _ => return Err(RunBlocked::CompileRequired),
        };
        let entry_path = self
            .config
            .as_ref()
            .and_then(|config| config.entry.as_ref())
            .ok_or(RunBlocked::EntryModuleMissing)?;
        let entry_id = graph
            .modules()
            .find(|image| image.path() == entry_path)
            .map(|image| image.id())
            .ok_or(RunBlocked::EntryModuleMissing)?;
        let entry_name = self
            .config
            .as_ref()
            .expect("a successful compile requires configuration")
            .run_entry
            .clone();

        for image in graph.modules() {
            self.runtime.load(image.clone());
        }

        let exit_code = self
            .runtime
            .run_module_entry(entry_id, entry_name.as_str(), args)
            .map_err(|error| RunBlocked::RuntimeError(error.to_string()))?;
        Ok(RunReport { exit_code })
    }
}
