use std::collections::HashMap;

use galfus_core::{Diagnostic, NodeId, SymbolId, TypeId};

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

        let expected = self.describe_type_for_diagnostic(expected);
        let actual = self.describe_type_for_diagnostic(actual);

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

    pub(super) fn report_invalid_enum_base_type(&mut self, base: NodeId, base_type: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(base)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let base_type = self.layer.table().describe(base_type);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidEnumBaseType,
            format!("enum base type must be an integer, got `{base_type}`"),
            span,
        ));
    }

    pub(super) fn report_missing_return(&mut self, function: NodeId, expected: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(function)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let expected = self.describe_type_for_diagnostic(expected);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::MissingReturn,
            format!("function must return `{expected}` on every path"),
            span,
        ));
    }

    pub(super) fn report_return_outside_function(&mut self, return_statement: NodeId) {
        let span = self
            .graph
            .syntax()
            .node(return_statement)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::ReturnOutsideFunction,
            "`return` can only be used inside a function",
            span,
        ));
    }

    pub(super) fn report_initialization_cycle(&mut self, binding: NodeId, name: &str) {
        let span = self
            .graph
            .syntax()
            .node(binding)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InitializationCycle,
            format!("binding `{name}` participates in an initialization cycle"),
            span,
        ));
    }

    pub(super) fn report_invalid_struct_expansion_target(
        &mut self,
        expansion: NodeId,
        actual: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(expansion)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let actual = self.describe_type_for_diagnostic(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidStructExpansionTarget,
            format!("struct expansion target must be a struct, got `{actual}`"),
            span,
        ));
    }

    pub(super) fn report_invalid_struct_spread_target(&mut self, spread: NodeId, actual: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(spread)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let actual = self.describe_type_for_diagnostic(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidStructSpreadTarget,
            format!("struct literal spread target must be a struct, got `{actual}`"),
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

    pub(super) fn report_unknown_struct_field(
        &mut self,
        field: NodeId,
        field_name: &str,
        struct_name: &str,
    ) {
        let span = self
            .graph
            .syntax()
            .node(field)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::UnknownStructField,
            format!("struct `{struct_name}` has no field `{field_name}`"),
            span,
        ));
    }

    pub(super) fn report_duplicate_struct_field(&mut self, field: NodeId, field_name: &str) {
        let span = self
            .graph
            .syntax()
            .node(field)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::DuplicateStructField,
            format!("duplicate field `{field_name}` in struct literal"),
            span,
        ));
    }

    pub(super) fn report_missing_struct_field(
        &mut self,
        literal: NodeId,
        field_name: &str,
        struct_name: &str,
    ) {
        let span = self
            .graph
            .syntax()
            .node(literal)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::MissingStructField,
            format!("missing required field `{field_name}` for struct `{struct_name}`"),
            span,
        ));
    }

    pub(super) fn report_invalid_struct_literal_target(
        &mut self,
        target: NodeId,
        target_name: &str,
    ) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidStructLiteralTarget,
            format!("struct literal target `{target_name}` is not a struct"),
            span,
        ));
    }

    pub(super) fn report_choice_payload_required(&mut self, target: NodeId, variant_name: &str) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::ChoicePayloadRequired,
            format!("choice variant `{variant_name}` requires a payload"),
            span,
        ));
    }

    pub(super) fn report_choice_payload_not_allowed(&mut self, target: NodeId, variant_name: &str) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::ChoicePayloadNotAllowed,
            format!("choice variant `{variant_name}` does not accept a payload"),
            span,
        ));
    }

    pub(super) fn report_argument_count_mismatch(
        &mut self,
        call: NodeId,
        expected: usize,
        actual: usize,
    ) {
        let span = self
            .graph
            .syntax()
            .node(call)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::ArgumentCountMismatch,
            format!("expected {expected} arguments, got {actual}"),
            span,
        ));
    }

    pub(super) fn report_invalid_condition_type(&mut self, condition: NodeId, actual: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(condition)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let actual = self.layer.table().describe(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidConditionType,
            format!("condition must be `bool`, got `{actual}`"),
            span,
        ));
    }

    pub(super) fn report_break_outside_loop(&mut self, node: NodeId) {
        let span = self
            .graph
            .syntax()
            .node(node)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::BreakOutsideLoop,
            "`break` can only be used inside a loop",
            span,
        ));
    }

    pub(super) fn report_continue_outside_loop(&mut self, node: NodeId) {
        let span = self
            .graph
            .syntax()
            .node(node)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::ContinueOutsideLoop,
            "`continue` can only be used inside a loop",
            span,
        ));
    }

    pub(super) fn report_invalid_iterable_type(&mut self, iterable: NodeId, actual: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(iterable)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let actual = self.layer.table().describe(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidIterableType,
            format!("for iterable must satisfy `Iterable`, got `{actual}`"),
            span,
        ));
    }

    pub(super) fn report_invalid_match_pattern_type(
        &mut self,
        pattern: NodeId,
        expected: TypeId,
        actual: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(pattern)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let expected = self.layer.table().describe(expected);
        let actual = self.layer.table().describe(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidMatchPatternType,
            format!("match pattern must be compatible with `{expected}`, got `{actual}`"),
            span,
        ));
    }

    pub(super) fn report_incompatible_match_arm_type(
        &mut self,
        body: NodeId,
        expected: TypeId,
        actual: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(body)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let expected = self.layer.table().describe(expected);
        let actual = self.layer.table().describe(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::IncompatibleMatchArmType,
            format!("match arm body must be compatible with `{expected}`, got `{actual}`"),
            span,
        ));
    }

    pub(super) fn report_invalid_instanceof_pattern_type(
        &mut self,
        pattern: NodeId,
        expected: TypeId,
        actual: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(pattern)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let expected = self.layer.table().describe(expected);
        let actual = self.layer.table().describe(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidInstanceofPatternType,
            format!("instanceof pattern must be compatible with `{expected}`, got `{actual}`"),
            span,
        ));
    }

    pub(super) fn report_incompatible_instanceof_arm_type(
        &mut self,
        body: NodeId,
        expected: TypeId,
        actual: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(body)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let expected = self.layer.table().describe(expected);
        let actual = self.layer.table().describe(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::IncompatibleInstanceofArmType,
            format!("instanceof arm body must be compatible with `{expected}`, got `{actual}`"),
            span,
        ));
    }

    pub(super) fn report_invalid_satisfies_target(&mut self, target: NodeId, target_name: &str) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidSatisfiesTarget,
            format!("satisfies target `{target_name}` is not a constraint"),
            span,
        ));
    }

    pub(super) fn report_missing_constraint_field(
        &mut self,
        item: NodeId,
        struct_name: &str,
        constraint_name: &str,
        field_name: &str,
    ) {
        let span = self
            .graph
            .syntax()
            .node(item)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
        TypeDiagnosticCode::MissingConstraintField,
        format!(
            "struct `{struct_name}` does not satisfy `{constraint_name}`: missing field `{field_name}`"
        ),
        span,
    ));
    }

    pub(super) fn report_constraint_field_type_mismatch(
        &mut self,
        field: NodeId,
        struct_name: &str,
        constraint_name: &str,
        field_name: &str,
        expected: TypeId,
        actual: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(field)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let expected = self.describe_type_for_diagnostic(expected);
        let actual = self.describe_type_for_diagnostic(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
        TypeDiagnosticCode::ConstraintFieldTypeMismatch,
        format!(
            "struct `{struct_name}` does not satisfy `{constraint_name}`: field `{field_name}` expected `{expected}`, got `{actual}`"
        ),
        span,
    ));
    }

    pub(super) fn report_missing_constraint_function(
        &mut self,
        item: NodeId,
        struct_name: &str,
        constraint_name: &str,
        function_name: &str,
    ) {
        let span = self
            .graph
            .syntax()
            .node(item)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
        TypeDiagnosticCode::MissingConstraintFunction,
        format!(
            "struct `{struct_name}` does not satisfy `{constraint_name}`: missing function `{function_name}`"
        ),
        span,
    ));
    }

    pub(super) fn report_constraint_function_type_mismatch(
        &mut self,
        function: NodeId,
        struct_name: &str,
        constraint_name: &str,
        function_name: &str,
        expected: TypeId,
        actual: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(function)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let expected = self.describe_type_for_diagnostic(expected);
        let actual = self.describe_type_for_diagnostic(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
        TypeDiagnosticCode::ConstraintFunctionTypeMismatch,
        format!(
            "struct `{struct_name}` does not satisfy `{constraint_name}`: function `{function_name}` expected `{expected}`, got `{actual}`"
        ),
        span,
    ));
    }

    pub(super) fn report_constraint_generic_argument_count_mismatch(
        &mut self,
        target: NodeId,
        constraint_name: &str,
        expected: usize,
        actual: usize,
    ) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
        TypeDiagnosticCode::ConstraintGenericArgumentCountMismatch,
        format!(
            "constraint `{constraint_name}` expects {expected} generic argument(s), got {actual}"
        ),
        span,
    ));
    }

    pub(super) fn report_invalid_range_operand_type(
        &mut self,
        operand: NodeId,
        expected: &str,
        actual: TypeId,
    ) {
        let span = self
            .graph
            .syntax()
            .node(operand)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let actual = self.layer.table().describe(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidRangeOperandType,
            format!("range operand must be {expected}, got `{actual}`"),
            span,
        ));
    }

    pub(super) fn report_generic_argument_count_mismatch(
        &mut self,
        target: NodeId,
        expected: usize,
        actual: usize,
    ) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::GenericArgumentCountMismatch,
            format!("expected {expected} generic argument(s), got {actual}"),
            span,
        ));
    }

    pub(super) fn report_call_argument_count_mismatch(
        &mut self,
        call: NodeId,
        function: &crate::FunctionType,
        argument_count: usize,
    ) {
        let parameters = function.parameters();

        let required_count = parameters
            .iter()
            .filter(|parameter| !parameter.has_default() && !parameter.is_rest())
            .count();

        let has_rest = parameters.iter().any(|parameter| parameter.is_rest());

        let max_count = if has_rest {
            None
        } else {
            Some(parameters.len())
        };

        let expected = match max_count {
            Some(max_count) if required_count == max_count => required_count.to_string(),
            Some(max_count) => format!("{required_count}..{max_count}"),
            None => format!("{required_count}+"),
        };

        let span = self
            .graph
            .syntax()
            .node(call)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::ArgumentCountMismatch,
            format!("expected {expected} arguments, got {argument_count}"),
            span,
        ));
    }

    pub(super) fn report_omitted_required_argument(&mut self, argument: NodeId) {
        let span = self
            .graph
            .syntax()
            .node(argument)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::ArgumentCountMismatch,
            "argument cannot be omitted because the matching parameter has no default",
            span,
        ));
    }

    pub(super) fn report_non_exhaustive_match(
        &mut self,
        match_expression: NodeId,
        subject_type: TypeId,
        missing_variants: &[String],
    ) {
        let span = self
            .graph
            .syntax()
            .node(match_expression)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let subject_type = self.describe_type_for_diagnostic(subject_type);
        let missing = missing_variants
            .iter()
            .map(|variant| format!("`{variant}`"))
            .collect::<Vec<_>>()
            .join(", ");

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::NonExhaustiveMatch,
            format!("match over `{subject_type}` is not exhaustive; missing {missing}"),
            span,
        ));
    }

    pub(super) fn report_invalid_decorator_usage(
        &mut self,
        node: NodeId,
        message: impl Into<String>,
    ) {
        let span = self
            .graph
            .syntax()
            .node(node)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::InvalidDecoratorUsage,
            message.into(),
            span,
        ));
    }

    pub(super) fn report_recursive_function_stamp(
        &mut self,
        symbol: SymbolId,
        path: &[SymbolId],
        stamp_functions: &HashMap<SymbolId, NodeId>,
    ) {
        let node = stamp_functions.get(&symbol).copied();
        let span = node
            .and_then(|node| self.graph.syntax().node(node).map(|node| node.span()))
            .unwrap_or_else(|| self.source.span());

        let path = self.describe_stamp_cycle(symbol, path);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::RecursiveFunctionStamp,
            format!("function stamp cannot be recursive through `{path}`"),
            span,
        ));
    }

    fn describe_stamp_cycle(&self, start: SymbolId, path: &[SymbolId]) -> String {
        let mut names = path
            .iter()
            .filter_map(|symbol| self.graph.resolution()?.symbol(*symbol))
            .map(|symbol| symbol.name().to_string())
            .collect::<Vec<_>>();

        if let Some(start_name) = self
            .graph
            .resolution()
            .and_then(|resolution| resolution.symbol(start))
            .map(|symbol| symbol.name().to_string())
        {
            names.push(start_name);
        }

        names.join(" -> ")
    }
}
