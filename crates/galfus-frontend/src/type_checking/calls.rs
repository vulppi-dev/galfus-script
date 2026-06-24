use galfus_core::{NodeId, TypeId};

use crate::{SyntaxNodeKind, TypeKind};

use super::DeclarationTypeChecker;

#[derive(Debug, Clone, Copy)]
enum CallArgument {
    Provided { expression: NodeId },
    Omitted { node: NodeId },
}

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_call_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let target = self.graph.syntax().child(node, 0)?;

        if self.is_choice_variant_call_target(target) {
            return self.infer_choice_variant_call_type(node);
        }

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

        let call_arguments = self.call_arguments(arguments);
        self.check_call_arguments(node, &function, call_arguments.as_slice());

        Some(function.return_type())
    }

    pub(super) fn call_argument_nodes(&self, arguments: NodeId) -> Vec<NodeId> {
        let Some(arguments_node) = self.graph.syntax().node(arguments) else {
            return Vec::new();
        };

        arguments_node
            .children()
            .iter()
            .filter_map(|child| {
                let syntax_node = self.graph.syntax().node(*child)?;

                match syntax_node.kind() {
                    SyntaxNodeKind::Argument => self.graph.syntax().child(*child, 0),
                    _ => Some(*child),
                }
            })
            .collect()
    }

    fn rest_parameter_element_type(&self, rest_type: TypeId) -> Option<TypeId> {
        match self.layer.table().kind(rest_type) {
            Some(TypeKind::Array { element }) => Some(*element),

            Some(TypeKind::FixedArray { element, .. }) => Some(*element),

            Some(TypeKind::Error) => Some(rest_type),

            _ => Some(rest_type),
        }
    }

    fn call_arguments(&self, arguments: NodeId) -> Vec<CallArgument> {
        let Some(arguments_node) = self.graph.syntax().node(arguments) else {
            return Vec::new();
        };

        arguments_node
            .children()
            .iter()
            .filter_map(|child| self.call_argument(*child))
            .collect()
    }

    fn call_argument(&self, node: NodeId) -> Option<CallArgument> {
        let syntax_node = self.graph.syntax().node(node)?;

        match syntax_node.kind() {
            SyntaxNodeKind::Argument => {
                let expression = self.graph.syntax().child(node, 0)?;
                Some(CallArgument::Provided { expression })
            }
            SyntaxNodeKind::OmittedArgument => Some(CallArgument::Omitted { node }),
            _ => Some(CallArgument::Provided { expression: node }),
        }
    }

    fn check_call_arguments(
        &mut self,
        call: NodeId,
        function: &crate::FunctionType,
        arguments: &[CallArgument],
    ) {
        let parameters = function.parameters();
        let mut parameter_index = 0;

        for argument in arguments.iter().copied() {
            let Some(parameter) = parameters.get(parameter_index) else {
                self.report_call_argument_count_mismatch(call, function, arguments.len());
                return;
            };

            if parameter.is_rest() {
                self.check_rest_call_argument(argument, parameter.ty());
                continue;
            }

            match argument {
                CallArgument::Provided { expression } => {
                    self.check_single_call_argument_type(expression, parameter.ty());
                    parameter_index += 1;
                }
                CallArgument::Omitted { node } => {
                    if !parameter.has_default() {
                        self.report_omitted_required_argument(node);
                    }

                    parameter_index += 1;
                }
            }
        }

        for parameter in parameters.iter().skip(parameter_index) {
            if parameter.is_rest() {
                return;
            }

            if parameter.has_default() {
                continue;
            }

            self.report_call_argument_count_mismatch(call, function, arguments.len());
            return;
        }
    }

    fn check_rest_call_argument(&mut self, argument: CallArgument, rest_type: TypeId) {
        match argument {
            CallArgument::Provided { expression } => {
                let expected = self
                    .rest_parameter_element_type(rest_type)
                    .unwrap_or(rest_type);

                self.check_single_call_argument_type(expression, expected);
            }
            CallArgument::Omitted { node } => {
                self.report_omitted_required_argument(node);
            }
        }
    }

    fn check_single_call_argument_type(&mut self, expression: NodeId, expected: TypeId) {
        let Some(actual) = self.infer_expression_type(expression) else {
            return;
        };

        if self.is_assignable(expected, actual) {
            return;
        }

        self.report_type_mismatch(expression, expected, actual);
    }
}
