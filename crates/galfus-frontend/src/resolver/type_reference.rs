use super::*;
use galfus_core::{Diagnostic, NodeId, ScopeId};

impl<'a> Resolver<'a> {
    pub(super) fn resolve_type_reference_item(&mut self, item: NodeId, module_scope: ScopeId) {
        let Some(node) = self.syntax.node(item) else {
            return;
        };

        match node.kind() {
            SyntaxNodeKind::ExportItem => {
                if let Some(inner) = node.first_child() {
                    self.resolve_type_reference_item(inner, module_scope);
                }
            }

            SyntaxNodeKind::FunctionItem => {
                self.resolve_function_type_references(item, module_scope);
            }

            SyntaxNodeKind::TypeAliasItem
            | SyntaxNodeKind::StructItem
            | SyntaxNodeKind::EnumItem
            | SyntaxNodeKind::ChoiceItem
            | SyntaxNodeKind::ConstraintItem => {
                let scope = self.resolution.node_scope(item).unwrap_or(module_scope);

                self.resolve_type_references_in_node(item, scope);
            }

            SyntaxNodeKind::VarItem | SyntaxNodeKind::ConstItem => {
                self.resolve_type_references_in_node(item, module_scope);
            }

            _ => {}
        }
    }

    fn resolve_function_type_references(&mut self, function: NodeId, module_scope: ScopeId) {
        let function_scope = self.resolution.node_scope(function).unwrap_or(module_scope);

        self.resolve_type_references_in_node(function, function_scope);
    }

    fn resolve_type_references_in_node(&mut self, node: NodeId, current_scope: ScopeId) {
        let Some(syntax_node) = self.syntax.node(node) else {
            return;
        };

        let scope = self.resolution.node_scope(node).unwrap_or(current_scope);

        match syntax_node.kind() {
            SyntaxNodeKind::NamedType => {
                self.resolve_named_type(node, scope);
                return;
            }

            SyntaxNodeKind::Path => {
                self.resolve_type_path(node, scope);
                return;
            }

            _ => {}
        }

        for child in syntax_node.children() {
            self.resolve_type_references_in_node(*child, scope);
        }
    }

    fn resolve_named_type(&mut self, named_type: NodeId, scope: ScopeId) {
        let Some(name) = self
            .syntax
            .first_child_of_kind(named_type, SyntaxNodeKind::Identifier)
        else {
            return;
        };

        let type_name = self.node_text(name);

        if let Some(symbol) = self.resolution.lookup_symbol(scope, type_name.as_str()) {
            let Some(symbol_data) = self.resolution.symbol(symbol) else {
                return;
            };

            if self.is_type_symbol(symbol_data.kind()) {
                self.resolution.bind_type_reference(named_type, symbol);
                return;
            }

            self.report_unresolved_type(name, type_name);
            return;
        }

        if let Some(symbol) = self.resolution.builtin_type_symbol(type_name.as_str()) {
            self.resolution.bind_type_reference(named_type, symbol);
            return;
        }

        self.report_unresolved_type(name, type_name);
    }

    fn resolve_type_path(&mut self, path: NodeId, scope: ScopeId) {
        let Some(root) = self
            .syntax
            .first_child_of_kind(path, SyntaxNodeKind::Identifier)
        else {
            return;
        };

        let root_name = self.node_text(root);

        let Some(symbol) = self.resolution.lookup_symbol(scope, root_name.as_str()) else {
            self.report_unresolved_type(root, root_name);
            return;
        };

        let Some(symbol_data) = self.resolution.symbol(symbol) else {
            return;
        };

        if self.is_type_path_root_symbol(symbol_data.kind()) {
            self.resolution.bind_type_reference(path, symbol);
            return;
        }

        self.report_unresolved_type(root, root_name);
    }

    fn is_type_symbol(&self, kind: SymbolKind) -> bool {
        matches!(
            kind,
            SymbolKind::TypeAlias
                | SymbolKind::Struct
                | SymbolKind::Enum
                | SymbolKind::Choice
                | SymbolKind::Constraint
                | SymbolKind::GenericParameter
                | SymbolKind::ImportBinding
                | SymbolKind::BuiltinType
        )
    }

    fn is_type_path_root_symbol(&self, kind: SymbolKind) -> bool {
        self.is_type_symbol(kind) || matches!(kind, SymbolKind::ImportNamespace)
    }

    fn report_unresolved_type(&mut self, name: NodeId, type_name: String) {
        let Some(name_node) = self.syntax.node(name) else {
            return;
        };

        self.diagnostics.push(Diagnostic::error_with_message(
            ResolverDiagnosticCode::UnresolvedType,
            format!("unresolved type `{type_name}`"),
            name_node.span(),
        ));
    }
}
