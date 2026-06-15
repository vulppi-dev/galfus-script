#[cfg(test)]
mod tests;

mod resolution;
mod scope;
mod symbol;

use galfus_core::{Diagnostic, DiagnosticBag, NodeId, ScopeId, SourceFile, SymbolId};

pub use resolution::*;
pub use scope::*;
pub use symbol::*;

use crate::{ModuleGraph, ResolverDiagnosticCode, SyntaxLayer, SyntaxNodeKind};

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

        let Some(root_node) = self.syntax.node(root) else {
            return;
        };

        for item in root_node.children() {
            self.declare_top_level_item(*item, module_scope);
        }

        for item in root_node.children() {
            self.declare_import_item(*item, module_scope);
        }
    }

    fn declare_top_level_item(&mut self, item: NodeId, scope: ScopeId) {
        let Some(node) = self.syntax.node(item) else {
            return;
        };

        match node.kind() {
            SyntaxNodeKind::ExportItem => {
                if let Some(inner) = node.first_child() {
                    self.declare_top_level_item(inner, scope);
                }
            }

            SyntaxNodeKind::ImportItem => {
                // R1 intentionally ignores imports.
                // R2 will declare local import bindings.
            }

            SyntaxNodeKind::FunctionItem => {
                self.declare_named_item(item, SymbolKind::Function, scope);
            }

            SyntaxNodeKind::TypeAliasItem => {
                self.declare_named_item(item, SymbolKind::TypeAlias, scope);
            }

            SyntaxNodeKind::StructItem => {
                self.declare_named_item(item, SymbolKind::Struct, scope);
            }

            SyntaxNodeKind::EnumItem => {
                self.declare_named_item(item, SymbolKind::Enum, scope);
            }

            SyntaxNodeKind::ChoiceItem => {
                self.declare_named_item(item, SymbolKind::Choice, scope);
            }

            SyntaxNodeKind::ConstraintItem => {
                self.declare_named_item(item, SymbolKind::Constraint, scope);
            }

            SyntaxNodeKind::VarItem => {
                self.declare_binding_item(item, SymbolKind::Var, scope);
            }

            SyntaxNodeKind::ConstItem => {
                self.declare_binding_item(item, SymbolKind::Const, scope);
            }

            _ => {}
        }
    }
    fn declare_named_item(&mut self, item: NodeId, kind: SymbolKind, scope: ScopeId) {
        let Some(name) = self
            .syntax
            .first_child_of_kind(item, SyntaxNodeKind::Identifier)
        else {
            return;
        };

        let symbol_name = self.node_text(name);

        self.declare_symbol(symbol_name, kind, name, scope);
    }

    fn declare_binding_item(&mut self, item: NodeId, kind: SymbolKind, scope: ScopeId) {
        let Some(binding) = self
            .syntax
            .first_child_of_kind(item, SyntaxNodeKind::BindingPattern)
        else {
            return;
        };

        self.declare_binding_pattern(binding, kind, scope);
    }

    fn declare_binding_pattern(&mut self, pattern: NodeId, kind: SymbolKind, scope: ScopeId) {
        let Some(node) = self.syntax.node(pattern) else {
            return;
        };

        match node.kind() {
            SyntaxNodeKind::BindingPattern => {
                if let Some(inner) = node.first_child() {
                    self.declare_binding_pattern(inner, kind, scope);
                }
            }

            SyntaxNodeKind::Identifier => {
                let symbol_name = self.node_text(pattern);
                self.declare_symbol(symbol_name, kind, pattern, scope);
            }

            SyntaxNodeKind::StructBindingPattern => {
                for field in node.children() {
                    self.declare_binding_pattern(*field, kind, scope);
                }
            }

            SyntaxNodeKind::StructBindingField => match node.child_count() {
                0 => {}

                1 => {
                    if let Some(name) = node.first_child() {
                        let symbol_name = self.node_text(name);
                        self.declare_symbol(symbol_name, kind, name, scope);
                    }
                }

                _ => {
                    if let Some(alias_pattern) = node.child(1) {
                        self.declare_binding_pattern(alias_pattern, kind, scope);
                    }
                }
            },

            SyntaxNodeKind::TupleBindingPattern | SyntaxNodeKind::ArrayBindingPattern => {
                for child in node.children() {
                    self.declare_binding_pattern(*child, kind, scope);
                }
            }

            SyntaxNodeKind::RestBindingPattern => {
                if let Some(inner) = node.first_child() {
                    self.declare_binding_pattern(inner, kind, scope);
                }
            }

            _ => {}
        }
    }

    fn declare_symbol(
        &mut self,
        name: String,
        kind: SymbolKind,
        declaration: NodeId,
        scope: ScopeId,
    ) -> Option<SymbolId> {
        if let Some(existing) = self
            .resolution
            .scope(scope)
            .and_then(|scope| scope.symbol(name.as_str()))
        {
            let span = self
                .syntax
                .node(declaration)
                .map(|node| node.span())
                .unwrap_or_else(|| self.source.span());

            self.diagnostics.push(Diagnostic::error_with_message(
                ResolverDiagnosticCode::DuplicateSymbol,
                format!("duplicate symbol `{name}`"),
                span,
            ));

            return Some(existing);
        }

        let symbol = self
            .resolution
            .add_symbol(kind, name.clone(), declaration, scope);

        if let Some(scope) = self.resolution.scope_mut(scope) {
            scope.insert_symbol(name, symbol);
        }

        Some(symbol)
    }

    fn node_text(&self, node: NodeId) -> String {
        let Some(node) = self.syntax.node(node) else {
            return String::new();
        };

        self.source.slice(node.span()).unwrap_or("").to_string()
    }

    fn declare_import_item(&mut self, item: NodeId, scope: ScopeId) {
        let Some(node) = self.syntax.node(item) else {
            return;
        };

        if node.kind() != SyntaxNodeKind::ImportItem {
            return;
        }

        let Some(clause) = node.first_child() else {
            return;
        };

        self.declare_import_clause(clause, scope);
    }

    fn declare_import_clause(&mut self, clause: NodeId, scope: ScopeId) {
        let Some(node) = self.syntax.node(clause) else {
            return;
        };

        match node.kind() {
            SyntaxNodeKind::NamespaceImport => {
                self.declare_namespace_import(clause, scope);
            }

            SyntaxNodeKind::NamedImportList => {
                for import in node.children() {
                    self.declare_named_import(*import, scope);
                }
            }

            _ => {}
        }
    }

    fn declare_namespace_import(&mut self, namespace_import: NodeId, scope: ScopeId) {
        let Some(name) = self
            .syntax
            .first_child_of_kind(namespace_import, SyntaxNodeKind::Identifier)
        else {
            return;
        };

        let symbol_name = self.node_text(name);

        self.declare_symbol(symbol_name, SymbolKind::ImportNamespace, name, scope);
    }

    fn declare_named_import(&mut self, named_import: NodeId, scope: ScopeId) {
        let Some(node) = self.syntax.node(named_import) else {
            return;
        };

        if node.kind() != SyntaxNodeKind::NamedImport {
            return;
        }

        let declaration = if let Some(alias) = self
            .syntax
            .first_child_of_kind(named_import, SyntaxNodeKind::ImportAlias)
        {
            self.syntax
                .first_child_of_kind(alias, SyntaxNodeKind::Identifier)
        } else {
            self.syntax
                .first_child_of_kind(named_import, SyntaxNodeKind::Identifier)
        };

        let Some(declaration) = declaration else {
            return;
        };

        let symbol_name = self.node_text(declaration);

        self.declare_symbol(symbol_name, SymbolKind::ImportBinding, declaration, scope);
    }
}

pub fn resolve(source: &SourceFile, mut graph: ModuleGraph) -> ResolveResult {
    let resolver = Resolver::new(source, graph.syntax());
    let (resolution, diagnostics) = resolver.resolve();

    graph.extend_diagnostics(diagnostics.into_vec());
    graph.set_resolution(resolution);

    ResolveResult { graph }
}
