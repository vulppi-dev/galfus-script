use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{PrimitiveType, SymbolKind, SyntaxNodeKind, TypeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_match_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let subject = self.graph.syntax().child(node, 0)?;
        let arms = self.graph.syntax().child(node, 1)?;

        let subject_type = self.infer_expression_type(subject)?;

        let arm_nodes = self
            .graph
            .syntax()
            .node(arms)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        if arm_nodes.is_empty() {
            self.report_cannot_infer_type(node, "cannot infer type of empty match expression");

            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        }

        let mut arm_types = Vec::new();

        for arm in arm_nodes {
            let Some(arm_type) = self.check_match_arm_type(arm, subject_type) else {
                continue;
            };

            arm_types.push((arm, arm_type));
        }

        let Some((_, expected)) = arm_types
            .iter()
            .copied()
            .find(|(_, ty)| !self.is_error_type(*ty))
        else {
            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        };

        let mut has_error = false;

        for (arm, actual) in arm_types.iter().copied() {
            if self.is_error_type(actual) {
                has_error = true;
                continue;
            }

            if self.is_assignable(expected, actual) {
                continue;
            }

            let body = self.graph.syntax().child(arm, 1).unwrap_or(arm);
            self.report_incompatible_match_arm_type(body, expected, actual);
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

    fn check_match_arm_type(&mut self, arm: NodeId, subject_type: TypeId) -> Option<TypeId> {
        let pattern = self.graph.syntax().child(arm, 0)?;
        let body = self.graph.syntax().child(arm, 1)?;

        self.check_match_pattern_type(pattern, subject_type);

        self.infer_match_arm_body_type(body)
    }

    fn infer_match_arm_body_type(&mut self, body: NodeId) -> Option<TypeId> {
        let body_node = self.graph.syntax().node(body)?;

        if body_node.kind() == SyntaxNodeKind::Block {
            return Some(self.layer.table().primitive(PrimitiveType::Null));
        }

        self.infer_expression_type(body)
    }

    fn check_match_pattern_type(&mut self, pattern: NodeId, expected: TypeId) {
        let Some(pattern_node) = self.graph.syntax().node(pattern) else {
            return;
        };

        match pattern_node.kind() {
            SyntaxNodeKind::LiteralPattern => {
                self.check_literal_match_pattern_type(pattern, expected);
            }

            SyntaxNodeKind::BindingPattern => {
                self.bind_match_binding_pattern_type(pattern, expected);
            }

            SyntaxNodeKind::VariantPattern => {
                self.check_variant_match_pattern_type(pattern, expected);
            }

            _ => {}
        }
    }

    fn check_literal_match_pattern_type(&mut self, pattern: NodeId, expected: TypeId) {
        let Some(literal) = self.graph.syntax().child(pattern, 0) else {
            return;
        };

        let Some(actual) = self.infer_expression_type(literal) else {
            return;
        };

        if self.is_assignable(expected, actual) {
            return;
        }

        self.report_invalid_match_pattern_type(pattern, expected, actual);
    }

    fn bind_match_binding_pattern_type(&mut self, pattern: NodeId, ty: TypeId) {
        let symbols = self.declaration_symbols_in_node(pattern, &[SymbolKind::PatternBinding]);

        for symbol in symbols {
            self.layer.bind_symbol_type(symbol, ty);
        }

        self.layer.bind_node_type(pattern, ty);
    }

    fn check_variant_match_pattern_type(&mut self, pattern: NodeId, expected: TypeId) {
        let Some((owner_symbol, variant_symbol)) = self.variant_pattern_symbols(pattern) else {
            return;
        };

        let owner_type = self
            .layer
            .symbol_type(owner_symbol)
            .unwrap_or_else(|| self.layer.table_mut().intern_named(owner_symbol));

        if !self.is_assignable(expected, owner_type) {
            self.report_invalid_match_pattern_type(pattern, expected, owner_type);
            return;
        }

        let Some(resolution) = self.graph.resolution() else {
            return;
        };

        let Some(variant_data) = resolution.symbol(variant_symbol) else {
            return;
        };

        match variant_data.kind() {
            SymbolKind::EnumVariant => {
                self.check_enum_variant_pattern_payload(pattern);
            }

            SymbolKind::ChoiceVariant => {
                self.check_choice_variant_pattern_payload(pattern, owner_symbol, variant_symbol);
            }

            _ => {}
        }
    }

    fn check_enum_variant_pattern_payload(&mut self, pattern: NodeId) {
        if self
            .graph
            .syntax()
            .first_child_of_kind(pattern, SyntaxNodeKind::VariantPatternPayload)
            .is_none()
        {
            return;
        }

        self.report_argument_count_mismatch(pattern, 0, 1);
    }

    fn check_choice_variant_pattern_payload(
        &mut self,
        pattern: NodeId,
        owner_symbol: SymbolId,
        variant_symbol: SymbolId,
    ) {
        let payload_types = self.choice_variant_payload_types(owner_symbol, variant_symbol);

        let payload_patterns = self
            .graph
            .syntax()
            .first_child_of_kind(pattern, SyntaxNodeKind::VariantPatternPayload)
            .and_then(|payload| {
                self.graph
                    .syntax()
                    .node(payload)
                    .map(|node| node.children().to_vec())
            })
            .unwrap_or_default();

        if payload_types.len() != payload_patterns.len() {
            self.report_argument_count_mismatch(
                pattern,
                payload_types.len(),
                payload_patterns.len(),
            );
            return;
        }

        for (payload_pattern, payload_type) in payload_patterns.into_iter().zip(payload_types) {
            self.check_match_pattern_type(payload_pattern, payload_type);
        }
    }

    fn variant_pattern_symbols(&self, pattern: NodeId) -> Option<(SymbolId, SymbolId)> {
        let resolution = self.graph.resolution()?;

        let owner_symbol = resolution.reference_symbol(pattern)?;
        let variant_symbol = resolution.path_reference_symbol(pattern)?;

        Some((owner_symbol, variant_symbol))
    }

    fn is_error_type(&self, ty: TypeId) -> bool {
        matches!(self.layer.table().kind(ty), Some(TypeKind::Error))
    }
}
