use galfus_core::{Diagnostic, NodeId, TypeId};

use crate::TypeDiagnosticCode;

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn report_type_mismatch(
        &mut self,
        expression: NodeId,
        expected: TypeId,
        actual: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(expression)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let expected = self.layer.table().describe(expected);
        let actual = self.layer.table().describe(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::TypeMismatch,
            format!("expected `{expected}`, got `{actual}`"),
            span,
        ));
    }

    pub(super) fn report_not_callable(&mut self, target: NodeId, target_type: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let actual = self.layer.table().describe(target_type);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::NotCallable,
            format!("type `{actual}` is not callable"),
            span,
        ));
    }

    pub(super) fn report_unsupported_operator(&mut self, operator: NodeId, operator_text: &str) {
        let span = self
            .graph
            .syntax()
            .node(operator)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::UnsupportedOperator,
            format!("unsupported operator `{operator_text}`"),
            span,
        ));
    }

    pub(super) fn report_operator_type_error(
        &mut self,
        operator: NodeId,
        expected: &str,
        left: TypeId,
        right: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(operator)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let left = self.layer.table().describe(left);
        let right = self.layer.table().describe(right);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::UnsupportedOperator,
            format!("operator requires {expected}, got `{left}` and `{right}`"),
            span,
        ));
    }

    pub(super) fn report_unary_operator_type_error(
        &mut self,
        operator: NodeId,
        expected: &str,
        operand: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(operator)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let operand = self.layer.table().describe(operand);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::UnsupportedOperator,
            format!("operator requires {expected}, got `{operand}`"),
            span,
        ));
    }

    pub(super) fn report_assignment_to_immutable(&mut self, target: NodeId, name: &str) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::AssignmentToImmutable,
            format!("cannot assign to immutable binding `{name}`"),
            span,
        ));
    }

    pub(super) fn report_unknown_member(
        &mut self,
        member: NodeId,
        member_name: &str,
        target_type: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(member)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let target_type = self.layer.table().describe(target_type);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::UnknownMember,
            format!("type `{target_type}` has no member `{member_name}`"),
            span,
        ));
    }

    pub(super) fn report_invalid_index_target(&mut self, target: NodeId, target_type: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let target_type = self.layer.table().describe(target_type);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidIndexTarget,
            format!("type `{target_type}` cannot be indexed"),
            span,
        ));
    }

    pub(super) fn report_invalid_index_type(&mut self, index: NodeId, index_type: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(index)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let index_type = self.layer.table().describe(index_type);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidIndexType,
            format!("index must be an integer, got `{index_type}`"),
            span,
        ));
    }

    pub(super) fn report_invalid_spread_target(&mut self, spread: NodeId, spread_type: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(spread)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let spread_type = self.layer.table().describe(spread_type);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidSpreadTarget,
            format!("spread target must be an array, got `{spread_type}`"),
            span,
        ));
    }

    pub(super) fn report_cannot_infer_type(&mut self, node: NodeId, message: impl Into<String>) {
        let span = self
            .graph
            .syntax()
            .node(node)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::CannotInferType,
            message.into(),
            span,
        ));
    }

    pub(super) fn report_empty_array_literal(&mut self, array: NodeId) {
        let span = self
            .graph
            .syntax()
            .node(array)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::EmptyArrayLiteral,
            "empty array literal is not allowed",
            span,
        ));
    }

    pub(super) fn report_dynamic_spread_in_array_literal(
        &mut self,
        spread: NodeId,
        spread_type: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(spread)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let spread_type = self.layer.table().describe(spread_type);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::DynamicSpreadInArrayLiteral,
            format!("array literal spread must have a known fixed size, got `{spread_type}`"),
            span,
        ));
    }
}
