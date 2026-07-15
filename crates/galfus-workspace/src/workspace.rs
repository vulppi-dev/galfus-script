use crate::config::{WorkspaceConfig, parse_workspace_config};
use crate::source_store::{ModuleOrigin, SourceStore};
use crate::state::{CheckState, CompileBlocked, CompileState, WorkspaceError};
use galfus_compiler::{
    CompiledImportEdge, CompiledModuleGraph, CompiledModuleImage, CompilerInput,
    input::CompiledModule,
};
use galfus_core::{DiagnosticBag, ModuleId, ModulePath, Revision, SourceFile, SourceId};
use galfus_frontend::modules::{FrontendRoots, FrontendSession, FrontendUpdate};
use std::sync::Arc;

pub struct Workspace {
    sources: SourceStore,
    config: Option<WorkspaceConfig>,
    revision: Revision,
    check_state: CheckState,
    compile_state: CompileState,
    frontend: FrontendSession,
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

impl Workspace {
    pub fn new() -> Self {
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

        self.revision.next();
        self.sources.load_module(
            module_path,
            Arc::from(module_bytes),
            ModuleOrigin::User,
            self.revision,
        );
        self.mark_dirty();
        Ok(LoadResult::Success)
    }

    pub fn remove_module(&mut self, path: &str) -> Result<RemoveResult, WorkspaceError> {
        let module_path = ModulePath::new(path).ok_or(WorkspaceError::InvalidPath)?;

        if self.sources.remove_module(&module_path).is_some() {
            self.revision.next();
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
            semantic_revision, ..
        } = &self.compile_state
        {
            let rev = *semantic_revision;
            self.compile_state = CompileState::Stale {
                last_successful_revision: Some(rev),
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
                let mut sources = Vec::new();
                for (id, entry) in self.sources.iter().enumerate() {
                    let source_id = SourceId::new(id as u32);
                    let text = std::str::from_utf8(&entry.bytes).unwrap_or("").to_string();
                    let source = SourceFile::new(source_id, entry.path.to_string(), text);
                    sources.push((entry.path.clone(), source));
                }

                let roots = FrontendRoots {};

                let update = FrontendUpdate {
                    source_revision: self.revision,
                    sources: &sources,
                    roots: &roots,
                };

                let report = self.frontend.check(update);

                if report.diagnostics.has_errors() {
                    self.check_state = CheckState::Failed {
                        revision: report.source_revision,
                        diagnostics: report.diagnostics,
                    };
                } else {
                    self.check_state = CheckState::Passed {
                        revision: report.source_revision,
                        semantic_revision: report.semantic_revision,
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
        let semantic_revision = match &self.check_state {
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
                semantic_revision, ..
            } => *semantic_revision,
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

        // Build CompiledModule list from the frontend's semantic modules.
        let semantic_modules = &self.frontend.modules;
        let mut compiled_modules: Vec<CompiledModule> = semantic_modules
            .iter()
            .map(|m| {
                CompiledModule::new(
                    m.path().clone(),
                    m.source().clone(),
                    m.graph().clone(),
                    m.type_result().cloned(),
                )
            })
            .collect();

        // Find the entry module.
        let entry_index = self
            .config
            .as_ref()
            .and_then(|cfg| cfg.entry())
            .and_then(|entry_path| compiled_modules.iter().position(|m| m.path() == entry_path))
            .ok_or(CompileBlocked::MissingConfiguration)?;

        let image_name = compiled_modules
            .get(entry_index)
            .map(|m| m.path().to_string())
            .unwrap_or_default();

        let mut input = CompilerInput {
            modules: compiled_modules.as_mut_slice(),
            entry_index,
            image_name: image_name.clone(),
        };

        let module_image = galfus_compiler::compile_to_image(&mut input)
            .map_err(|e| CompileBlocked::CompilerError(e.to_string()))?;

        // Build the CompiledModuleGraph from the result.
        // Phase 5 will produce one image per module; for now we put the single
        // monolithic image under the entry module's ID.
        let entry_module_id = ModuleId::new(entry_index as u32);
        let entry_module_path = compiled_modules[entry_index].path().clone();

        let compiled_image = CompiledModuleImage {
            id: entry_module_id,
            path: entry_module_path,
            semantic_revision,
            image: module_image,
        };

        // Build import edges from the semantic graph.
        let edges: Vec<CompiledImportEdge> = self
            .frontend
            .modules
            .iter()
            .flat_map(|m| {
                let from = m.id();
                let graph = m.graph();
                let resolution = match graph.resolution() {
                    Some(r) => r,
                    None => return vec![],
                };
                resolution
                    .imports()
                    .iter()
                    .filter_map(|_import| {
                        // Phase 9 will resolve these to ModuleIds properly.
                        // For now edges are not populated (no multi-module images yet).
                        None::<CompiledImportEdge>
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut module_graph = CompiledModuleGraph::new();
        module_graph.upsert(compiled_image);
        module_graph.set_edges(edges);

        let graph = Arc::new(module_graph);
        self.compile_state = CompileState::Ready {
            semantic_revision,
            graph: Arc::clone(&graph),
        };

        Ok(CompileReport { graph })
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}
