use galfus_core::{NodeId, TypeId};

use crate::SyntaxNodeKind;

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_range_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let start = self.graph.syntax().child(node, 0)?;
        let end_or_count = self.graph.syntax().child(node, 2)?;

        let start_type = self.infer_expression_type(start)?;
        let end_or_count_type = self.infer_expression_type(end_or_count)?;

        let start_is_numeric = self.is_numeric_type(start_type);
        let end_or_count_is_numeric = self.is_numeric_type(end_or_count_type);

        let mut has_error = false;

        if !start_is_numeric {
            self.report_invalid_range_operand_type(start, "numeric", start_type);
            has_error = true;
        }

        if !end_or_count_is_numeric {
            self.report_invalid_range_operand_type(end_or_count, "numeric", end_or_count_type);
            has_error = true;
        } else if start_is_numeric && !self.is_same_numeric_type(start_type, end_or_count_type) {
            let expected = format!(
                "same numeric type as range start `{}`",
                self.layer.table().describe(start_type)
            );

            self.report_invalid_range_operand_type(
                end_or_count,
                expected.as_str(),
                end_or_count_type,
            );

            has_error = true;
        }

        if let Some(step) = self.graph.syntax().child(node, 3) {
            if let Some(step_type) = self.infer_range_step_type(step) {
                let step_is_numeric = self.is_numeric_type(step_type);

                if !step_is_numeric {
                    self.report_invalid_range_operand_type(step, "numeric", step_type);
                    has_error = true;
                } else if start_is_numeric && !self.is_same_numeric_type(start_type, step_type) {
                    let expected = format!(
                        "same numeric type as range start `{}`",
                        self.layer.table().describe(start_type)
                    );

                    self.report_invalid_range_operand_type(step, expected.as_str(), step_type);
                    has_error = true;
                }
            }
        }

        if has_error {
            return Some(self.layer.table_mut().error());
        }

        Some(self.layer.table_mut().intern_range(start_type))
    }

    fn infer_range_step_type(&mut self, step: NodeId) -> Option<TypeId> {
        let syntax_node = self.graph.syntax().node(step)?;

        if syntax_node.kind() == SyntaxNodeKind::RangeStep {
            let expression = self.graph.syntax().child(step, 0)?;
            return self.infer_expression_type(expression);
        }

        self.infer_expression_type(step)
    }
}
