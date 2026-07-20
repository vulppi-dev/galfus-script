use crate::BytecodeModule;
use galfus_core::{ModuleId, ModulePath, SemanticRevision};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// The compiled artifact for a single module.
///
/// Each `BytecodeNode` records which `SemanticRevision` it was produced
/// from. The workspace can use this to skip recompilation when a module's
/// semantic result has not changed since the last compile cycle.
#[derive(Debug, Clone, Default)]
pub struct ExecutionMetadata {
    pub spans: HashMap<crate::instruction::FuncIdx, HashMap<usize, galfus_core::Span>>,
}

#[derive(Debug, Clone)]
pub struct BytecodeNode {
    pub id: ModuleId,
    /// Logical path — stable identifier used for cross-module linking.
    pub path: ModulePath,
    /// The semantic revision of the frontend output this image was compiled from.
    pub semantic_revision: SemanticRevision,
    /// The compiled bytecode image.
    pub image: BytecodeModule,
    pub metadata: Option<ExecutionMetadata>,
}

impl BytecodeNode {
    pub fn id(&self) -> ModuleId {
        self.id
    }

    pub fn path(&self) -> &ModulePath {
        &self.path
    }

    pub fn semantic_revision(&self) -> SemanticRevision {
        self.semantic_revision
    }

    pub fn image(&self) -> &BytecodeModule {
        &self.image
    }
}

/// An edge in the compiled module dependency graph.
///
/// `from` imports something from `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportEdge {
    pub from: ModuleId,
    pub to: ModuleId,
}

#[derive(Debug, Clone)]
pub struct BytecodeGraphTransaction {
    pub upserted_modules: Vec<BytecodeNode>,
    pub removed_modules: Vec<ModuleId>,
    pub edges: Vec<ImportEdge>,
}

/// The full compiled representation of a workspace.
///
/// The workspace holds one of these after a successful `compile()`. Individual
/// modules can be upserted independently when the compiler detects that only a
/// subset of modules changed (incremental compilation — Phase 10).
#[derive(Debug, Clone, Default)]
pub struct BytecodeGraph {
    pub(crate) modules: HashMap<ModuleId, BytecodeNode>,
    pub(crate) ids_by_path: HashMap<ModulePath, ModuleId>,
    pub(crate) edges: Vec<ImportEdge>,
}

impl BytecodeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a compiled module image.
    pub fn upsert(&mut self, image: BytecodeNode) {
        self.modules.insert(image.id, image);
    }

    /// Remove a module from the graph.
    pub fn remove(&mut self, id: ModuleId) -> Option<BytecodeNode> {
        self.modules.remove(&id)
    }

    /// Applies a transaction atomically to the graph.
    pub fn apply(&mut self, transaction: BytecodeGraphTransaction) {
        for id in transaction.removed_modules {
            self.modules.remove(&id);
        }
        for image in transaction.upserted_modules {
            self.modules.insert(image.id, image);
        }
        self.edges = transaction.edges;
    }

    pub fn get(&self, id: ModuleId) -> Option<&BytecodeNode> {
        self.modules.get(&id)
    }

    pub fn modules(&self) -> impl Iterator<Item = &BytecodeNode> {
        self.modules.values()
    }

    pub fn edges(&self) -> &[ImportEdge] {
        self.edges.as_slice()
    }

    /// Direct dependencies (imports) of `id`.
    pub fn deps_of(&self, id: ModuleId) -> impl Iterator<Item = ModuleId> + '_ {
        self.edges
            .iter()
            .filter(move |e| e.from == id)
            .map(|e| e.to)
    }

    /// Modules that transitively depend on `id` (reverse reachability).
    ///
    /// Used to determine which modules must be recompiled when `id` changes.
    pub fn dependents_of(&self, id: ModuleId) -> Vec<ModuleId> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.collect_dependents(id, &mut result, &mut visited);
        result
    }

    fn collect_dependents(
        &self,
        id: ModuleId,
        out: &mut Vec<ModuleId>,
        visited: &mut std::collections::HashSet<ModuleId>,
    ) {
        for edge in &self.edges {
            if edge.to == id && visited.insert(edge.from) {
                out.push(edge.from);
                self.collect_dependents(edge.from, out, visited);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }
}
