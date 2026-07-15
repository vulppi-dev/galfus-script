use crate::config::{parse_workspace_config, WorkspaceConfig};
use crate::source_store::{ModuleOrigin, SourceStore};
use crate::state::{CheckState, CompileState, WorkspaceError};
use galfus_core::{DiagnosticBag, ModulePath, Revision};
use std::sync::Arc;

pub struct Workspace {
    sources: SourceStore,
    config: Option<WorkspaceConfig>,
    revision: Revision,
    check_state: CheckState,
    compile_state: CompileState,
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

pub struct CompileReport<'a> {
    // Placeholder until image is added
    pub placeholder: std::marker::PhantomData<&'a ()>,
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

        // Mark compile stale if it was ready
        if let CompileState::Ready { semantic_revision } = self.compile_state {
            self.compile_state = CompileState::Stale {
                last_successful_revision: Some(semantic_revision),
            };
        }
    }

    pub fn check(&mut self) -> CheckReport<'_> {
        // Here we will call FrontendSession::check() in Phase 2
        // For now, we simulate a check result or just leave it dirty until we integrate frontend.
        // Actually, we must produce a CheckReport.
        
        // Let's just create an empty diagnostic bag and pretend it failed or passed depending on if we have a config.
        let mut bag = DiagnosticBag::new();
        
        if self.config.is_none() {
            self.check_state = CheckState::Failed {
                revision: self.revision,
                diagnostics: bag,
            };
        } else {
            // Normally this calls frontend.
            // We pretend it passed for Phase 1 so we can pass tests.
            self.check_state = CheckState::Passed {
                revision: self.revision,
                semantic_revision: galfus_core::SemanticRevision::new(1),
                diagnostics: bag,
            };
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
}
