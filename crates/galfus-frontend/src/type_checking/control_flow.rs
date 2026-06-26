use galfus_core::{NodeId, TypeId};

use crate::{PrimitiveType, SymbolKind, SyntaxNodeKind, TypeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn check_control_flow(&mut self, node: NodeId, loop_depth: usize) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        match syntax_node.kind() {
            SyntaxNodeKind::IfStatement => {
                self.check_if_statement_control_flow(node, loop_depth);
            }

            SyntaxNodeKind::WhileStatement => {
                self.check_while_statement_control_flow(node, loop_depth);
            }

            SyntaxNodeKind::LoopStatement => {
                self.check_loop_statement_control_flow(node, loop_depth);
            }

            SyntaxNodeKind::ForStatement => {
                self.check_for_statement_control_flow(node, loop_depth);
            }

            SyntaxNodeKind::BreakStatement => {
                if loop_depth == 0 {
                    self.report_break_outside_loop(node);
                }
            }

            SyntaxNodeKind::ContinueStatement => {
                if loop_depth == 0 {
                    self.report_continue_outside_loop(node);
                }
            }

            _ => {
                let children = syntax_node.children().to_vec();

                for child in children {
                    self.check_control_flow(child, loop_depth);
                }
            }
        }
    }

    fn check_if_statement_control_flow(&mut self, node: NodeId, loop_depth: usize) {
        let Some(condition) = self.graph.syntax().child(node, 0) else {
            return;
        };

        self.check_bool_condition(condition);

        let children = self
            .graph
            .syntax()
            .node(node)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        for child in children.into_iter().skip(1) {
            self.check_control_flow(child, loop_depth);
        }
    }

    fn check_while_statement_control_flow(&mut self, node: NodeId, loop_depth: usize) {
        let Some(condition) = self.graph.syntax().child(node, 0) else {
            return;
        };

        self.check_bool_condition(condition);

        if let Some(body) = self.graph.syntax().child(node, 1) {
            self.check_control_flow(body, loop_depth + 1);
        }
    }

    fn check_loop_statement_control_flow(&mut self, node: NodeId, loop_depth: usize) {
        if let Some(body) = self.graph.syntax().child(node, 0) {
            self.check_control_flow(body, loop_depth + 1);
        }
    }

    fn check_for_statement_control_flow(&mut self, node: NodeId, loop_depth: usize) {
        let Some(binding) = self.graph.syntax().child(node, 0) else {
            return;
        };

        let Some(iterable) = self.graph.syntax().child(node, 1) else {
            return;
        };

        let Some(body) = self.graph.syntax().child(node, 2) else {
            return;
        };

        let Some(element_type) = self.check_for_iterable_type(iterable) else {
            self.check_control_flow(body, loop_depth + 1);
            return;
        };

        self.bind_for_binding_type(binding, element_type);
        self.check_control_flow(body, loop_depth + 1);
    }

    fn check_bool_condition(&mut self, condition: NodeId) {
        let Some(actual) = self.infer_expression_type(condition) else {
            return;
        };

        if self.is_bool_type(actual) {
            return;
        }

        self.report_invalid_condition_type(condition, actual);
    }

    fn is_bool_type(&self, ty: galfus_core::TypeId) -> bool {
        matches!(
            self.layer.table().kind(ty),
            Some(TypeKind::Primitive(PrimitiveType::Bool)) | Some(TypeKind::Error)
        )
    }

    fn check_for_iterable_type(&mut self, iterable: NodeId) -> Option<TypeId> {
        let actual = self.infer_expression_type(iterable)?;

        if let Some(element_type) = self.iterable_item_type(actual) {
            return Some(element_type);
        }

        self.report_invalid_iterable_type(iterable, actual);
        None
    }

    fn bind_for_binding_type(&mut self, binding: NodeId, element_type: TypeId) {
        let symbols = self.declaration_symbols_in_node(binding, &[SymbolKind::ForBinding]);

        for symbol in symbols {
            self.layer.bind_symbol_type(symbol, element_type);
        }

        self.layer.bind_node_type(binding, element_type);
    }
}
