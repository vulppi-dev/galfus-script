use galfus_core::{NodeId, TypeId};

use crate::{PrimitiveType, SymbolKind, SyntaxNodeKind, TypeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_instanceof_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let subject = self.graph.syntax().child(node, 0)?;
        let arms = self.graph.syntax().child(node, 1)?;

        let subject_type = self.infer_expression_type(subject)?;
        let subject_symbol = self.instanceof_subject_reference_symbol(subject);
        let subject_text = self.node_text(subject);

        let arm_nodes = self
            .graph
            .syntax()
            .node(arms)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        if arm_nodes.is_empty() {
            self.report_cannot_infer_type(node, "cannot infer type of empty instanceof expression");

            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        }

        let mut arm_types = Vec::new();

        for arm in arm_nodes {
            let Some(arm_type) = self.check_instanceof_arm_type(
                arm,
                subject_type,
                subject_symbol,
                subject_text.as_str(),
            ) else {
                continue;
            };

            arm_types.push((arm, arm_type));
        }

        let Some((_, expected)) = arm_types
            .iter()
            .copied()
            .find(|(_, ty)| !self.is_instanceof_error_type(*ty))
        else {
            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        };

        let mut has_error = false;

        for (arm, actual) in arm_types.iter().copied() {
            if self.is_instanceof_error_type(actual) {
                has_error = true;
                continue;
            }

            if self.is_assignable(expected, actual) {
                continue;
            }

            let body = self.graph.syntax().child(arm, 1).unwrap_or(arm);
            self.report_incompatible_instanceof_arm_type(body, expected, actual);
            has_error = true;
        }

        let ty = if has_error {
            self.layer.table_mut().error()
        } else {
            expected
        };

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    fn check_instanceof_arm_type(
        &mut self,
        arm: NodeId,
        subject_type: TypeId,
        subject_symbol: Option<galfus_core::SymbolId>,
        subject_text: &str,
    ) -> Option<TypeId> {
        let pattern = self.graph.syntax().child(arm, 0)?;
        let body = self.graph.syntax().child(arm, 1)?;

        let narrowed_type = self.check_instanceof_pattern_type(pattern, subject_type);

        self.infer_instanceof_arm_body_type_with_narrowing(
            body,
            subject_symbol,
            subject_text,
            narrowed_type,
        )
    }

    fn infer_instanceof_arm_body_type(&mut self, body: NodeId) -> Option<TypeId> {
        let body_node = self.graph.syntax().node(body)?;

        if body_node.kind() == SyntaxNodeKind::Block {
            return Some(self.layer.table().primitive(PrimitiveType::Null));
        }

        self.infer_expression_type(body)
    }

    fn check_instanceof_pattern_type(
        &mut self,
        pattern: NodeId,
        subject_type: TypeId,
    ) -> Option<TypeId> {
        let Some(pattern_node) = self.graph.syntax().node(pattern) else {
            return None;
        };

        match pattern_node.kind() {
            SyntaxNodeKind::TypePattern => {
                self.check_type_instanceof_pattern_type(pattern, subject_type)
            }

            SyntaxNodeKind::BindingPattern => {
                self.bind_instanceof_binding_pattern_type(pattern, subject_type);
                Some(subject_type)
            }

            SyntaxNodeKind::WildcardPattern => {
                self.layer.bind_node_type(pattern, subject_type);
                Some(subject_type)
            }

            _ => None,
        }
    }

    fn check_type_instanceof_pattern_type(
        &mut self,
        pattern: NodeId,
        subject_type: TypeId,
    ) -> Option<TypeId> {
        let Some(type_node) = self.first_type_child(pattern) else {
            return None;
        };

        let Some(pattern_type) = self.layer.node_type(type_node) else {
            return None;
        };

        if !self.is_instanceof_pattern_compatible(subject_type, pattern_type) {
            self.report_invalid_instanceof_pattern_type(pattern, subject_type, pattern_type);
            return None;
        }

        self.bind_instanceof_type_pattern_binding(pattern, pattern_type);
        self.layer.bind_node_type(pattern, pattern_type);

        Some(pattern_type)
    }

    fn infer_instanceof_arm_body_type_with_narrowing(
        &mut self,
        body: NodeId,
        subject_symbol: Option<galfus_core::SymbolId>,
        subject_text: &str,
        narrowed_type: Option<TypeId>,
    ) -> Option<TypeId> {
        let Some(subject_symbol) = subject_symbol else {
            return self.infer_instanceof_arm_body_type(body);
        };
        let Some(narrowed_type) = narrowed_type else {
            return self.infer_instanceof_arm_body_type(body);
        };

        if self.instanceof_subject_reference_symbol(body) == Some(subject_symbol)
            || self.node_text(body) == subject_text
        {
            self.layer.bind_node_type(body, narrowed_type);
            return Some(narrowed_type);
        }

        let previous = self.layer.symbol_type(subject_symbol);
        self.layer.bind_symbol_type(subject_symbol, narrowed_type);

        let ty = self.infer_instanceof_arm_body_type(body);

        if let Some(previous) = previous {
            self.layer.bind_symbol_type(subject_symbol, previous);
        }

        ty
    }

    fn instanceof_subject_reference_symbol(
        &self,
        expression: NodeId,
    ) -> Option<galfus_core::SymbolId> {
        let resolution = self.graph.resolution()?;

        resolution.reference_symbol(expression).or_else(|| {
            let identifier = self
                .graph
                .syntax()
                .first_child_of_kind(expression, SyntaxNodeKind::Identifier)?;

            resolution.reference_symbol(identifier)
        })
    }

    fn bind_instanceof_type_pattern_binding(&mut self, pattern: NodeId, ty: TypeId) {
        let Some(binding) = self
            .graph
            .syntax()
            .first_child_of_kind(pattern, SyntaxNodeKind::TypePatternBinding)
        else {
            return;
        };

        let symbols = self.declaration_symbols_in_node(binding, &[SymbolKind::TypePatternBinding]);

        for symbol in symbols {
            self.layer.bind_symbol_type(symbol, ty);
        }

        self.layer.bind_node_type(binding, ty);
    }

    fn bind_instanceof_binding_pattern_type(&mut self, pattern: NodeId, ty: TypeId) {
        let symbols = self.declaration_symbols_in_node(pattern, &[SymbolKind::PatternBinding]);

        for symbol in symbols {
            self.layer.bind_symbol_type(symbol, ty);
        }

        self.layer.bind_node_type(pattern, ty);
    }

    fn is_instanceof_pattern_compatible(&self, subject_type: TypeId, pattern_type: TypeId) -> bool {
        if self.is_assignable(subject_type, pattern_type) {
            return true;
        }

        if self.is_assignable(pattern_type, subject_type) {
            return true;
        }

        self.union_contains_type(subject_type, pattern_type)
    }

    fn union_contains_type(&self, union_type: TypeId, member_type: TypeId) -> bool {
        let union_type = self.resolve_alias_type(union_type);
        let member_type = self.resolve_alias_type(member_type);

        match self.layer.table().kind(union_type) {
            Some(TypeKind::Union { members }) => members
                .iter()
                .copied()
                .any(|member| self.is_assignable(member, member_type)),

            Some(TypeKind::Error) => true,

            _ => false,
        }
    }

    fn is_instanceof_error_type(&self, ty: TypeId) -> bool {
        matches!(self.layer.table().kind(ty), Some(TypeKind::Error))
    }
}
