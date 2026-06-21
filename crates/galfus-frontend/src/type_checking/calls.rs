use galfus_core::{Diagnostic, NodeId, TypeId};

use crate::{SyntaxNodeKind, TypeDiagnosticCode, TypeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_call_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let target = self.graph.syntax().child(node, 0)?;
        let arguments = self.graph.syntax().child(node, 1)?;

        let target_type = self.infer_expression_type(target)?;

        let function = match self.layer.table().kind(target_type).cloned() {
            Some(TypeKind::Function(function)) => function,
            Some(TypeKind::Error) => return Some(self.layer.table_mut().error()),
            _ => {
                self.report_not_callable(target, target_type);
                return Some(self.layer.table_mut().error());
            }
        };

        let argument_nodes = self.call_argument_nodes(arguments);

        self.check_call_argument_count(node, &function, argument_nodes.len());

        self.check_call_argument_types(argument_nodes.as_slice(), &function);

        Some(function.return_type())
    }

    fn call_argument_nodes(&self, arguments: NodeId) -> Vec<NodeId> {
        let Some(arguments_node) = self.graph.syntax().node(arguments) else {
            return Vec::new();
        };

        arguments_node
            .children()
            .iter()
            .filter_map(|child| self.call_argument_expression(*child))
            .collect()
    }

    fn call_argument_expression(&self, node: NodeId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        match syntax_node.kind() {
            SyntaxNodeKind::Argument | SyntaxNodeKind::SpreadArgument => {
                self.graph.syntax().child(node, 0)
            }

            _ => Some(node),
        }
    }

    fn check_call_argument_count(
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

        let too_few = argument_count < required_count;
        let too_many = max_count.is_some_and(|max_count| argument_count > max_count);

        if !too_few && !too_many {
            return;
        }

        let expected = match max_count {
            Some(max_count) if required_count == max_count => required_count.to_string(),
            Some(max_count) => {
                format!("{required_count}..{max_count}")
            }
            None => {
                format!("{required_count}+")
            }
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

    fn check_call_argument_types(
        &mut self,
        argument_nodes: &[NodeId],
        function: &crate::FunctionType,
    ) {
        for (index, argument) in argument_nodes.iter().copied().enumerate() {
            let Some(expected) = self.call_parameter_type(function, index) else {
                continue;
            };

            let Some(actual) = self.infer_expression_type(argument) else {
                continue;
            };

            if self.is_assignable(expected, actual) {
                continue;
            }

            self.report_type_mismatch(argument, expected, actual);
        }
    }

    fn call_parameter_type(
        &self,
        function: &crate::FunctionType,
        argument_index: usize,
    ) -> Option<TypeId> {
        let parameters = function.parameters();

        if let Some(parameter) = parameters.get(argument_index) {
            if parameter.is_rest() {
                return self.rest_parameter_element_type(parameter.ty());
            }

            return Some(parameter.ty());
        }

        let rest = parameters.iter().find(|parameter| parameter.is_rest())?;

        self.rest_parameter_element_type(rest.ty())
    }

    fn rest_parameter_element_type(&self, rest_type: TypeId) -> Option<TypeId> {
        match self.layer.table().kind(rest_type) {
            Some(TypeKind::Array { element }) => Some(*element),

            Some(TypeKind::FixedArray { element, .. }) => Some(*element),

            Some(TypeKind::Error) => Some(rest_type),

            _ => Some(rest_type),
        }
    }
}
