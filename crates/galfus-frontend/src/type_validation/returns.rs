use galfus_core::{NodeId, TypeId};

use crate::{PrimitiveType, SyntaxNodeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn check_return_types(&mut self, node: NodeId, current_return_type: Option<TypeId>) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        match syntax_node.kind() {
            SyntaxNodeKind::FunctionItem => {
                let function_return_type = self
                    .last_direct_type_child(node)
                    .and_then(|return_type| self.layer.node_type(return_type));

                for child in syntax_node.children() {
                    self.check_return_types(*child, function_return_type);
                }

                return;
            }

            SyntaxNodeKind::ArrowFunctionExpression => {
                return;
            }

            SyntaxNodeKind::ReturnStatement => {
                self.check_return_statement_type(node, current_return_type);
            }

            _ => {}
        }

        for child in syntax_node.children() {
            self.check_return_types(*child, current_return_type);
        }
    }

    fn check_return_statement_type(&mut self, return_statement: NodeId, expected: Option<TypeId>) {
        let Some(expected) = expected else {
            return;
        };

        let actual = match self.graph.syntax().child(return_statement, 0) {
            Some(expression) => {
                match self.infer_expression_type_with_expected(expression, Some(expected)) {
                    Some(actual) => actual,
                    None => return,
                }
            }

            None => self.layer.table().primitive(PrimitiveType::Null),
        };

        if self.is_assignable(expected, actual) {
            return;
        }

        let diagnostic_node = self
            .graph
            .syntax()
            .child(return_statement, 0)
            .unwrap_or(return_statement);

        self.report_type_mismatch(diagnostic_node, expected, actual);
    }
}
