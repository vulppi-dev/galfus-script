use galfus_core::NodeId;

use crate::SyntaxNodeKind;

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn check_decorators(&mut self, node: NodeId) {
        self.check_decorators_in_node(node);
    }

    fn check_decorators_in_node(&mut self, node: NodeId) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        let kind = syntax_node.kind();
        let children = syntax_node.children().to_vec();

        for child in children.iter().copied() {
            let Some(child_node) = self.graph.syntax().node(child) else {
                continue;
            };

            if child_node.kind() != SyntaxNodeKind::DecoratorList {
                continue;
            }

            if self.can_own_decorators(kind) {
                self.check_decorator_list(child);
            } else {
                self.report_invalid_decorator_usage(
                    child,
                    format!("decorators are not valid on `{kind:?}`"),
                );
            }
        }

        for child in children {
            let Some(child_node) = self.graph.syntax().node(child) else {
                continue;
            };

            if child_node.kind() == SyntaxNodeKind::DecoratorList {
                continue;
            }

            self.check_decorators_in_node(child);
        }
    }

    fn can_own_decorators(&self, kind: SyntaxNodeKind) -> bool {
        matches!(
            kind,
            SyntaxNodeKind::FunctionItem
                | SyntaxNodeKind::StructItem
                | SyntaxNodeKind::StructField
                | SyntaxNodeKind::WeakStructField
                | SyntaxNodeKind::Parameter
                | SyntaxNodeKind::RestParameter
                | SyntaxNodeKind::ChoicePayloadItem
        )
    }

    fn check_decorator_list(&mut self, list: NodeId) {
        let decorators = self
            .graph
            .syntax()
            .node(list)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        for decorator in decorators {
            self.check_decorator(decorator);
        }
    }

    fn check_decorator(&mut self, decorator: NodeId) {
        let Some(target) = self.graph.syntax().child(decorator, 0) else {
            self.report_invalid_decorator_usage(decorator, "decorator requires a target");
            return;
        };

        self.check_decorator_target(target);
    }

    fn check_decorator_target(&mut self, target: NodeId) {
        let Some(target_node) = self.graph.syntax().node(target) else {
            return;
        };

        match target_node.kind() {
            SyntaxNodeKind::NameExpression | SyntaxNodeKind::PathExpression => {}
            SyntaxNodeKind::CallExpression => {
                self.check_decorator_call_target(target);
                self.check_decorator_call_arguments(target);
            }
            _ => {
                self.report_invalid_decorator_usage(
                    target,
                    "decorator target must be a name, path, or call",
                );
            }
        }
    }

    fn check_decorator_call_target(&mut self, call: NodeId) {
        let Some(callee) = self.graph.syntax().child(call, 0) else {
            return;
        };

        let Some(callee_node) = self.graph.syntax().node(callee) else {
            return;
        };

        if matches!(
            callee_node.kind(),
            SyntaxNodeKind::NameExpression | SyntaxNodeKind::PathExpression
        ) {
            return;
        }

        self.report_invalid_decorator_usage(callee, "decorator call target must be a name or path");
    }

    fn check_decorator_call_arguments(&mut self, call: NodeId) {
        let Some(arguments) = self
            .graph
            .syntax()
            .first_child_of_kind(call, SyntaxNodeKind::ArgumentList)
        else {
            return;
        };

        let argument_nodes = self
            .graph
            .syntax()
            .node(arguments)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        for argument in argument_nodes {
            let Some(argument_node) = self.graph.syntax().node(argument) else {
                continue;
            };

            match argument_node.kind() {
                SyntaxNodeKind::Argument => {
                    let Some(expression) = self.graph.syntax().child(argument, 0) else {
                        continue;
                    };

                    self.infer_expression_type(expression);
                }
                SyntaxNodeKind::OmittedArgument => {
                    self.report_invalid_decorator_usage(
                        argument,
                        "decorator arguments cannot be omitted",
                    );
                }
                _ => {
                    self.infer_expression_type(argument);
                }
            }
        }
    }
}
