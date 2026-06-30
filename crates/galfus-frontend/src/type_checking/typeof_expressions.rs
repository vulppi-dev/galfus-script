use std::collections::HashMap;

use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{PrimitiveType, SyntaxNodeKind, TypeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_typeof_expression_type(
        &mut self,
        node: NodeId,
        expected: Option<TypeId>,
    ) -> Option<TypeId> {
        let subject = self.graph.syntax().child(node, 0)?;
        let arms = self.graph.syntax().child(node, 1)?;

        let subject_type = self.layer.node_type(subject)?;
        let effective_subject_type = self.typeof_effective_subject_type(subject_type);

        let arm_nodes = self
            .graph
            .syntax()
            .node(arms)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        if arm_nodes.is_empty() {
            self.report_cannot_infer_type(node, "cannot infer type of empty typeof expression");

            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        }

        let subject_generic = self.generic_parameter_symbol(subject_type);
        let mut arm_types = Vec::new();

        for arm in arm_nodes {
            let Some((pattern_type, arm_type)) =
                self.check_typeof_arm_type(arm, effective_subject_type, subject_generic, expected)
            else {
                continue;
            };

            arm_types.push((arm, pattern_type, arm_type));
        }

        if let Some(expected) = expected {
            let mut has_error = false;

            for (arm, pattern_type, actual) in arm_types.iter().copied() {
                if self.is_typeof_error_type(actual) {
                    has_error = true;
                    continue;
                }

                let branch_expected =
                    self.branch_expected_type(expected, subject_generic, pattern_type);

                if self.is_assignable(branch_expected, actual) {
                    continue;
                }

                let body = self.graph.syntax().child(arm, 1).unwrap_or(arm);
                self.report_incompatible_typeof_arm_type(body, branch_expected, actual);
                has_error = true;
            }

            let ty = if has_error {
                self.layer.table_mut().error()
            } else {
                expected
            };

            self.layer.bind_node_type(node, ty);
            return Some(ty);
        }

        let Some((_, _, expected)) = arm_types
            .iter()
            .copied()
            .find(|(_, _, ty)| !self.is_typeof_error_type(*ty))
        else {
            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        };

        let mut has_error = false;

        for (arm, _, actual) in arm_types.iter().copied() {
            if self.is_typeof_error_type(actual) {
                has_error = true;
                continue;
            }

            if self.is_assignable(expected, actual) {
                continue;
            }

            let body = self.graph.syntax().child(arm, 1).unwrap_or(arm);
            self.report_incompatible_typeof_arm_type(body, expected, actual);
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

    fn check_typeof_arm_type(
        &mut self,
        arm: NodeId,
        subject_type: TypeId,
        subject_generic: Option<SymbolId>,
        expected: Option<TypeId>,
    ) -> Option<(TypeId, TypeId)> {
        let pattern = self.graph.syntax().child(arm, 0)?;
        let body = self.graph.syntax().child(arm, 1)?;

        let pattern_type = self.check_typeof_pattern_type(pattern, subject_type)?;
        let expected = expected
            .map(|expected| self.branch_expected_type(expected, subject_generic, pattern_type));

        let arm_type = self.infer_typeof_arm_body_type_with_narrowing(
            body,
            subject_generic,
            pattern_type,
            expected,
        );

        Some((pattern_type, arm_type?))
    }

    fn check_typeof_pattern_type(
        &mut self,
        pattern: NodeId,
        subject_type: TypeId,
    ) -> Option<TypeId> {
        let pattern_node = self.graph.syntax().node(pattern)?;

        match pattern_node.kind() {
            SyntaxNodeKind::TypePattern => {
                let type_node = self.first_type_child(pattern)?;
                let pattern_type = self.layer.node_type(type_node)?;

                if !self.is_typeof_pattern_compatible(subject_type, pattern_type) {
                    self.report_invalid_typeof_pattern_type(pattern, subject_type, pattern_type);
                    return None;
                }

                self.layer.bind_node_type(pattern, pattern_type);
                Some(pattern_type)
            }

            SyntaxNodeKind::WildcardPattern => {
                self.layer.bind_node_type(pattern, subject_type);
                Some(subject_type)
            }

            _ => None,
        }
    }

    fn infer_typeof_arm_body_type_with_narrowing(
        &mut self,
        body: NodeId,
        subject_generic: Option<SymbolId>,
        narrowed_type: TypeId,
        expected: Option<TypeId>,
    ) -> Option<TypeId> {
        let Some(subject_generic) = subject_generic else {
            return self.infer_typeof_arm_body_type(body, expected);
        };

        let mut substitution = HashMap::new();
        substitution.insert(subject_generic, narrowed_type);
        self.active_type_substitutions.push(substitution);

        let ty = self.infer_typeof_arm_body_type(body, expected);

        self.active_type_substitutions.pop();

        ty
    }

    fn infer_typeof_arm_body_type(
        &mut self,
        body: NodeId,
        expected: Option<TypeId>,
    ) -> Option<TypeId> {
        let body_node = self.graph.syntax().node(body)?;

        if body_node.kind() == SyntaxNodeKind::Block {
            return Some(self.layer.table().primitive(PrimitiveType::Null));
        }

        self.infer_expression_type_with_expected(body, expected)
    }

    fn typeof_effective_subject_type(&mut self, subject_type: TypeId) -> TypeId {
        let Some(symbol) = self.generic_parameter_symbol(subject_type) else {
            return subject_type;
        };

        self.generic_parameter_bound_type(symbol)
            .unwrap_or(subject_type)
    }

    fn generic_parameter_symbol(&self, ty: TypeId) -> Option<SymbolId> {
        let ty = self.resolve_alias_type(ty);

        match self.layer.table().kind(ty) {
            Some(TypeKind::GenericParameter { symbol }) => Some(*symbol),
            _ => None,
        }
    }

    fn is_typeof_pattern_compatible(&mut self, subject_type: TypeId, pattern_type: TypeId) -> bool {
        let subject_type = self.resolve_alias_type(subject_type);
        let pattern_type = self.resolve_alias_type(pattern_type);

        if self.is_assignable(subject_type, pattern_type)
            || self.is_assignable(pattern_type, subject_type)
        {
            return true;
        }

        match self.layer.table().kind(subject_type).cloned() {
            Some(TypeKind::Union { members }) => members
                .into_iter()
                .any(|member| self.is_typeof_pattern_compatible(member, pattern_type)),
            Some(TypeKind::Named { symbol }) if self.is_constraint_symbol(symbol) => {
                self.type_satisfies_generic_bound(pattern_type, subject_type)
            }
            Some(TypeKind::Error) => true,
            _ => false,
        }
    }

    fn branch_expected_type(
        &mut self,
        expected: TypeId,
        subject_generic: Option<SymbolId>,
        pattern_type: TypeId,
    ) -> TypeId {
        let Some(subject_generic) = subject_generic else {
            return expected;
        };

        let mut substitution = HashMap::new();
        substitution.insert(subject_generic, pattern_type);

        self.substitute_generic_expression_type(expected, &substitution)
    }

    pub(super) fn apply_active_type_substitutions(&mut self, ty: TypeId) -> TypeId {
        if self.active_type_substitutions.is_empty() {
            return ty;
        }

        let mut substitution = HashMap::new();

        for active in &self.active_type_substitutions {
            substitution.extend(active.iter().map(|(symbol, ty)| (*symbol, *ty)));
        }

        self.substitute_generic_expression_type(ty, &substitution)
    }

    fn is_typeof_error_type(&self, ty: TypeId) -> bool {
        matches!(self.layer.table().kind(ty), Some(TypeKind::Error))
    }
}
