use crate::config::{WorkspaceConfig, parse_workspace_config};
use crate::source_store::{ModuleOrigin, SourceStore};
use crate::state::{CheckState, CompileBlocked, CompileState, WorkspaceError};
use galfus_compiler::{
    CompiledImportEdge, CompiledModule, CompiledModuleGraph, CompiledModuleImage,
};
use galfus_core::{DiagnosticBag, ModulePath, Revision, SourceFile};
use galfus_frontend::modules::{FrontendRoots, FrontendSession, FrontendSource, FrontendUpdate};
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
                let source_files = self
                    .sources
                    .iter()
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
                let mut sources = Vec::new();
                for (module_id, path, source) in &source_files {
                    sources.push(FrontendSource {
                        module_id: *module_id,
                        path: path.clone(),
                        source,
                    });
                }

                let roots = FrontendRoots::default();

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

        // Compile each module individually — one ModuleImage per module.
        let outputs = galfus_compiler::compile_module_images(&mut compiled_modules)
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
        let mut module_graph = CompiledModuleGraph::new();
        for output in outputs {
            let mod_idx = output.module_id.raw() as usize;
            let path = compiled_modules
                .get(mod_idx)
                .map(|m| m.path().clone())
                .unwrap_or_else(|| galfus_core::ModulePath::new("unknown.gfs").unwrap());
            let module_rev = self
                .frontend
                .modules
                .get(mod_idx)
                .map(|m| m.semantic_revision())
                .unwrap_or(semantic_revision);
            module_graph.upsert(CompiledModuleImage {
                id: output.module_id,
                path,
                semantic_revision: module_rev,
                image: output.image,
            });
        }
        module_graph.set_edges(edges);

        let graph = Arc::new(module_graph);
        self.compile_state = CompileState::Ready {
            semantic_revision,
            graph: Arc::clone(&graph),
        };

        Ok(CompileReport { graph })
    }
}
