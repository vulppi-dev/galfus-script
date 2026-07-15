use galfus_core::{DiagnosticBag, Revision, SemanticRevision};
// use std::sync::Arc;
// use galfus_image::ModuleImage;

#[derive(Debug)]
pub enum CheckState {
    Dirty {
        current_revision: Revision,
        previous_checked_revision: Option<Revision>,
    },
    Passed {
        revision: Revision,
        semantic_revision: SemanticRevision,
        diagnostics: DiagnosticBag,
    },
    Failed {
        revision: Revision,
        diagnostics: DiagnosticBag,
    },
}

impl CheckState {
    pub fn is_dirty(&self) -> bool {
        matches!(self, Self::Dirty { .. })
    }

    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }
}

// CompileError and ModuleImage to be imported correctly when integrating compiler crate
#[derive(Debug)]
pub struct CompileError(pub String);

#[derive(Debug)]
pub enum CompileState {
    Missing,
    Stale {
        last_successful_revision: Option<SemanticRevision>,
    },
    Ready {
        semantic_revision: SemanticRevision,
        // image: Arc<ModuleImage>,
    },
    Failed {
        semantic_revision: SemanticRevision,
        error: CompileError,
    },
}

#[derive(Debug)]
pub enum CompileBlocked {
    Dirty {
        current_revision: Revision,
        checked_revision: Option<Revision>,
    },
    CheckFailed {
        revision: Revision,
        error_count: usize,
    },
    MissingConfiguration,
}

#[derive(Debug)]
pub enum WorkspaceError {
    InvalidPath,
    MissingConfiguration,
}
