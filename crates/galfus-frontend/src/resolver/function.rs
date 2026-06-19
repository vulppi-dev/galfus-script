use super::*;
use galfus_core::{NodeId, ScopeId};

impl<'a> Resolver<'a> {
    pub(super) fn resolve_function_scope_item(&mut self, item: NodeId, parent_scope: ScopeId) {
        let Some(node) = self.syntax.node(item) else {
            return;
        };

        match node.kind() {
            SyntaxNodeKind::ExportItem => {
                if let Some(inner) = node.first_child() {
                    self.resolve_function_scope_item(inner, parent_scope);
                }
            }

            SyntaxNodeKind::FunctionItem => {
                self.resolve_function_item_scope(item, parent_scope);
            }

            _ => {}
        }
    }

    fn resolve_function_item_scope(&mut self, function: NodeId, parent_scope: ScopeId) {
        let function_scope =
            self.resolution
                .add_scope(ScopeKind::Function, Some(parent_scope), Some(function));

        if let Some(parameters) = self
            .syntax
            .first_child_of_kind(function, SyntaxNodeKind::ParameterList)
        {
            self.resolution.bind_scope(parameters, function_scope);
            self.declare_parameter_list(parameters, function_scope);
        }
    }

    pub(super) fn declare_parameter_list(&mut self, parameters: NodeId, scope: ScopeId) {
        let Some(node) = self.syntax.node(parameters) else {
            return;
        };

        for parameter in node.children() {
            self.declare_parameter(*parameter, scope);
        }
    }

    fn declare_parameter(&mut self, parameter: NodeId, scope: ScopeId) {
        let Some(node) = self.syntax.node(parameter) else {
            return;
        };

        match node.kind() {
            SyntaxNodeKind::Parameter => {
                self.declare_parameter_symbol(parameter, SymbolKind::Parameter, scope);
            }

            SyntaxNodeKind::RestParameter => {
                self.declare_parameter_symbol(parameter, SymbolKind::RestParameter, scope);
            }

            _ => {}
        }
    }

    fn declare_parameter_symbol(&mut self, parameter: NodeId, kind: SymbolKind, scope: ScopeId) {
        let Some(name) = self
            .syntax
            .first_child_of_kind(parameter, SyntaxNodeKind::Identifier)
        else {
            return;
        };

        let symbol_name = self.node_text(name);

        self.declare_symbol(symbol_name, kind, name, scope);
    }
}
