use crate::{ModuleGraph, TypeCheckResult};
use galfus_core::{ModuleId, ModulePath, Revision, SourceFile, SourceId};

#[derive(Debug, Clone)]
pub struct SemanticModule {
    pub id: ModuleId,
    pub source_id: SourceId,
    pub path: ModulePath,
    pub source_revision: Revision,

    pub source: SourceFile,
    pub graph: ModuleGraph,
    pub type_result: Option<TypeCheckResult>,
}

impl SemanticModule {
    pub fn id(&self) -> ModuleId {
        self.id
    }

    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    pub fn path(&self) -> &ModulePath {
        &self.path
    }

    pub fn source_revision(&self) -> Revision {
        self.source_revision
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

    pub fn type_result_mut(&mut self) -> Option<&mut TypeCheckResult> {
        self.type_result.as_mut()
    }
}
