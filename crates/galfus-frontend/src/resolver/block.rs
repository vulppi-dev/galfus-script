use super::*;

impl<'a> Resolver<'a> {
    pub(super) fn resolve_block_scope_item(&mut self, item: NodeId, parent_scope: ScopeId) {
        let Some(node) = self.syntax.node(item) else {
            return;
        };

        match node.kind() {
            SyntaxNodeKind::ExportItem => {
                if let Some(inner) = node.first_child() {
                    self.resolve_block_scope_item(inner, parent_scope);
                }
            }

            SyntaxNodeKind::FunctionItem => {
                self.resolve_function_body_block(item);
            }

            _ => {}
        }
    }

    fn resolve_function_body_block(&mut self, function: NodeId) {
        let Some(function_scope) = self.resolution.node_scope(function) else {
            return;
        };

        let Some(block) = self
            .syntax
            .first_child_of_kind(function, SyntaxNodeKind::Block)
        else {
            return;
        };

        self.resolve_block(block, function_scope);
    }

    pub(super) fn resolve_block(&mut self, block: NodeId, parent_scope: ScopeId) -> ScopeId {
        let block_scope =
            self.resolution
                .add_scope(ScopeKind::Block, Some(parent_scope), Some(block));

        self.declare_block_statements(block, block_scope);

        block_scope
    }

    fn declare_block_statements(&mut self, block: NodeId, scope: ScopeId) {
        let Some(node) = self.syntax.node(block) else {
            return;
        };

        for statement in node.children() {
            self.declare_block_statement(*statement, scope);
        }
    }

    fn declare_block_statement(&mut self, statement: NodeId, scope: ScopeId) {
        let Some(node) = self.syntax.node(statement) else {
            return;
        };

        match node.kind() {
            SyntaxNodeKind::VarStatement => {
                self.declare_statement_binding(statement, SymbolKind::Var, scope);
            }

            SyntaxNodeKind::ConstStatement => {
                self.declare_statement_binding(statement, SymbolKind::Const, scope);
            }

            SyntaxNodeKind::Block => {
                self.resolve_block(statement, scope);
            }

            SyntaxNodeKind::IfStatement
            | SyntaxNodeKind::ForStatement
            | SyntaxNodeKind::WhileStatement
            | SyntaxNodeKind::LoopStatement
            | SyntaxNodeKind::MatchStatement
            | SyntaxNodeKind::InstanceofStatement => {
                self.resolve_nested_blocks_in(statement, scope);
            }

            _ => {}
        }
    }

    fn declare_statement_binding(&mut self, statement: NodeId, kind: SymbolKind, scope: ScopeId) {
        let Some(binding) = self
            .syntax
            .first_child_of_kind(statement, SyntaxNodeKind::BindingPattern)
        else {
            return;
        };

        self.declare_binding_pattern(binding, kind, scope);
    }

    fn resolve_nested_blocks_in(&mut self, node: NodeId, parent_scope: ScopeId) {
        let Some(syntax_node) = self.syntax.node(node) else {
            return;
        };

        for child in syntax_node.children() {
            let Some(child_node) = self.syntax.node(*child) else {
                continue;
            };

            match child_node.kind() {
                SyntaxNodeKind::Block => {
                    self.resolve_block(*child, parent_scope);
                }

                _ => {
                    self.resolve_nested_blocks_in(*child, parent_scope);
                }
            }
        }
    }
}
