#[cfg(test)]
mod tests;

mod resolution;
mod scope;
mod symbol;

use galfus_core::{DiagnosticBag, SourceFile};
pub use resolution::*;
pub use scope::*;
pub use symbol::*;

use crate::{ModuleGraph, SyntaxLayer};

pub struct ResolveResult {
    graph: ModuleGraph,
}

impl ResolveResult {
    pub fn graph(&self) -> &ModuleGraph {
        &self.graph
    }

    pub fn into_graph(self) -> ModuleGraph {
        self.graph
    }

    pub fn has_errors(&self) -> bool {
        self.graph.has_errors()
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        self.graph.diagnostics()
    }
}

pub struct Resolver<'a> {
    source: &'a SourceFile,
    syntax: &'a SyntaxLayer,
    diagnostics: DiagnosticBag,

    resolution: ResolutionLayer,
}

impl<'a> Resolver<'a> {
    pub fn new(source: &'a SourceFile, syntax: &'a SyntaxLayer) -> Self {
        let mut resolution = ResolutionLayer::new();
        resolution.add_scope(ScopeKind::Module, None, syntax.root());

        Self {
            source,
            syntax,
            diagnostics: DiagnosticBag::new(),
            resolution,
        }
    }

    pub fn resolve(mut self) -> (ResolutionLayer, DiagnosticBag) {
        self.resolve_source_file();

        (self.resolution, self.diagnostics)
    }

    fn resolve_source_file(&mut self) {
        let Some(root) = self.syntax.root() else {
            return;
        };

        let module_scope = self.resolution.module_scope();
        self.resolution.bind_scope(root, module_scope);

        let _ = self.source;
    }
}

pub fn resolve(source: &SourceFile, mut graph: ModuleGraph) -> ResolveResult {
    let resolver = Resolver::new(source, graph.syntax());
    let (resolution, diagnostics) = resolver.resolve();

    graph.extend_diagnostics(diagnostics.into_vec());
    graph.set_resolution(resolution);

    ResolveResult { graph }
}
