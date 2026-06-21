use galfus_core::NodeId;

use crate::SyntaxNodeKind;

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn check_initializer_types(&mut self, node: NodeId) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        match syntax_node.kind() {
            SyntaxNodeKind::VarItem
            | SyntaxNodeKind::ConstItem
            | SyntaxNodeKind::VarStatement
            | SyntaxNodeKind::ConstStatement => {
                self.check_binding_initializer_type(node);
            }

            _ => {}
        }

        for child in syntax_node.children() {
            self.check_initializer_types(*child);
        }
    }

    fn check_binding_initializer_type(&mut self, node: NodeId) {
        let Some(type_annotation) = self
            .graph
            .syntax()
            .first_child_of_kind(node, SyntaxNodeKind::TypeAnnotation)
        else {
            return;
        };

        let Some(type_node) = self.first_type_child(type_annotation) else {
            return;
        };

        let Some(expected) = self.layer.node_type(type_node) else {
            return;
        };

        let Some(initializer) = self
            .graph
            .syntax()
            .first_child_of_kind(node, SyntaxNodeKind::Initializer)
        else {
            return;
        };

        let Some(expression) = self.graph.syntax().child(initializer, 0) else {
            return;
        };

        let Some(actual) = self.infer_expression_type(expression) else {
            return;
        };

        if self.is_assignable(expected, actual) {
            return;
        }

        self.report_type_mismatch(expression, expected, actual);
    }
}
