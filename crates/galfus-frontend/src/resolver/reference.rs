use super::*;
use galfus_core::{NodeId, ScopeId};

impl<'a> Resolver<'a> {
    pub(super) fn resolve_reference_item(&mut self, item: NodeId, parent_scope: ScopeId) {
        let Some(node) = self.syntax.node(item) else {
            return;
        };

        match node.kind() {
            SyntaxNodeKind::ExportItem => {
                if let Some(inner) = node.first_child() {
                    self.resolve_reference_item(inner, parent_scope);
                }
            }

            SyntaxNodeKind::FunctionItem => {
                self.resolve_function_references(item);
            }

            SyntaxNodeKind::VarItem | SyntaxNodeKind::ConstItem => {
                self.resolve_node_references(item, parent_scope);
            }

            _ => {}
        }
    }

    fn resolve_function_references(&mut self, function: NodeId) {
        let Some(function_scope) = self.resolution.node_scope(function) else {
            return;
        };

        self.resolve_function_parameter_defaults(function, function_scope);

        let Some(block) = self
            .syntax
            .first_child_of_kind(function, SyntaxNodeKind::Block)
        else {
            return;
        };

        let block_scope = self.resolution.node_scope(block).unwrap_or(function_scope);

        self.resolve_node_references(block, block_scope);
    }

    fn resolve_function_parameter_defaults(&mut self, function: NodeId, function_scope: ScopeId) {
        let Some(parameters) = self
            .syntax
            .first_child_of_kind(function, SyntaxNodeKind::ParameterList)
        else {
            return;
        };

        self.resolve_node_references(parameters, function_scope);
    }

    fn resolve_node_references(&mut self, node: NodeId, current_scope: ScopeId) {
        let Some(syntax_node) = self.syntax.node(node) else {
            return;
        };

        let scope = self.resolution.node_scope(node).unwrap_or(current_scope);

        match syntax_node.kind() {
            SyntaxNodeKind::NameExpression => {
                self.resolve_name_expression(node, scope);
                return;
            }

            SyntaxNodeKind::PathExpression => {
                self.resolve_path_expression(node, scope);
                return;
            }

            SyntaxNodeKind::VariantPattern => {
                self.resolve_variant_pattern(node, scope);
            }

            // Nested functions, if allowed later, should own their own pass.
            SyntaxNodeKind::FunctionItem => {
                return;
            }

            _ => {}
        }

        for child in syntax_node.children() {
            self.resolve_node_references(*child, scope);
        }
    }

    fn resolve_name_expression(&mut self, expression: NodeId, scope: ScopeId) {
        let Some(name) = self
            .syntax
            .first_child_of_kind(expression, SyntaxNodeKind::Identifier)
        else {
            return;
        };

        let symbol_name = self.node_text(name);

        if let Some(symbol) = self.resolution.lookup_symbol(scope, symbol_name.as_str()) {
            self.resolution.bind_reference(expression, symbol);
            return;
        }

        let Some(name_node) = self.syntax.node(name) else {
            return;
        };

        self.diagnostics.push(Diagnostic::error_with_message(
            ResolverDiagnosticCode::UnresolvedName,
            format!("unresolved name `{symbol_name}`"),
            name_node.span(),
        ));
    }

    fn resolve_path_expression(&mut self, expression: NodeId, scope: ScopeId) {
        let Some(target) = self.syntax.first_child(expression) else {
            return;
        };

        self.resolve_node_references(target, scope);

        if let Some(symbol) = self.resolution.reference_symbol(target) {
            self.resolution.bind_reference(expression, symbol);
            self.resolve_path_expression_member(expression, symbol);
        }
    }

    fn resolve_path_expression_member(&mut self, expression: NodeId, root_symbol: SymbolId) {
        let Some(member) = self.syntax.child(expression, 1) else {
            return;
        };

        let Some(member_scope) = self.resolution.member_scope(root_symbol) else {
            return;
        };

        let member_name = self.node_text(member);

        if let Some(symbol) = self
            .resolution
            .scope(member_scope)
            .and_then(|scope| scope.symbol(member_name.as_str()))
        {
            self.resolution.bind_path_reference(expression, symbol);
            return;
        }

        self.report_unresolved_path_member(member, member_name);
    }

    fn resolve_variant_pattern(&mut self, pattern: NodeId, scope: ScopeId) {
        let Some(root) = self.syntax.child(pattern, 0) else {
            return;
        };

        let root_name = self.node_text(root);

        let Some(root_symbol) = self.resolution.lookup_symbol(scope, root_name.as_str()) else {
            self.report_unresolved_name(root, root_name);
            return;
        };

        self.resolution.bind_reference(pattern, root_symbol);

        let Some(variant) = self.syntax.child(pattern, 1) else {
            return;
        };

        self.resolve_variant_pattern_member(pattern, root_symbol, variant);
    }

    fn resolve_variant_pattern_member(
        &mut self,
        pattern: NodeId,
        root_symbol: SymbolId,
        variant: NodeId,
    ) {
        let Some(member_scope) = self.resolution.member_scope(root_symbol) else {
            return;
        };

        let variant_name = self.node_text(variant);

        let Some(symbol) = self
            .resolution
            .scope(member_scope)
            .and_then(|scope| scope.symbol(variant_name.as_str()))
        else {
            self.report_unresolved_path_member(variant, variant_name);
            return;
        };

        let Some(symbol_data) = self.resolution.symbol(symbol) else {
            return;
        };

        if matches!(
            symbol_data.kind(),
            SymbolKind::EnumVariant | SymbolKind::ChoiceVariant
        ) {
            self.resolution.bind_path_reference(pattern, symbol);
            return;
        }

        self.report_unresolved_path_member(variant, variant_name);
    }

    fn report_unresolved_name(&mut self, name: NodeId, symbol_name: String) {
        let Some(name_node) = self.syntax.node(name) else {
            return;
        };

        self.diagnostics.push(Diagnostic::error_with_message(
            ResolverDiagnosticCode::UnresolvedName,
            format!("unresolved name `{symbol_name}`"),
            name_node.span(),
        ));
    }

    fn report_unresolved_path_member(&mut self, name: NodeId, member_name: String) {
        let Some(name_node) = self.syntax.node(name) else {
            return;
        };

        self.diagnostics.push(Diagnostic::error_with_message(
            ResolverDiagnosticCode::UnresolvedName,
            format!("unresolved path member `{member_name}`"),
            name_node.span(),
        ));
    }
}
