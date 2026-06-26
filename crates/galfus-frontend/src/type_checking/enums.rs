use galfus_core::{NodeId, TypeId};

use crate::{PrimitiveType, SyntaxNodeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn check_enum_types(&mut self, node: NodeId) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        if syntax_node.kind() == SyntaxNodeKind::EnumItem {
            self.check_enum_item_type(node);
        }

        for child in syntax_node.children() {
            self.check_enum_types(*child);
        }
    }

    fn check_enum_item_type(&mut self, enum_item: NodeId) {
        let Some(base_type) = self.enum_base_type(enum_item) else {
            return;
        };

        if !self.is_integer_type(base_type.1) {
            self.report_invalid_enum_base_type(base_type.0, base_type.1);
        }

        let Some(variants) = self
            .graph
            .syntax()
            .first_child_of_kind(enum_item, SyntaxNodeKind::EnumVariantList)
        else {
            return;
        };

        let variant_nodes = self
            .graph
            .syntax()
            .node(variants)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        for variant in variant_nodes {
            self.check_enum_variant_discriminant_type(variant, base_type.1);
        }
    }

    fn enum_base_type(&mut self, enum_item: NodeId) -> Option<(NodeId, TypeId)> {
        let type_node = self
            .graph
            .syntax()
            .node(enum_item)?
            .children()
            .iter()
            .copied()
            .find(|child| {
                self.graph
                    .syntax()
                    .node(*child)
                    .is_some_and(|node| node.kind().is_type())
            });

        match type_node {
            Some(type_node) => self.layer.node_type(type_node).map(|ty| (type_node, ty)),
            None => {
                let int32 = self.layer.table().primitive(PrimitiveType::Int32);
                Some((enum_item, int32))
            }
        }
    }

    fn check_enum_variant_discriminant_type(&mut self, variant: NodeId, base_type: TypeId) {
        let Some(discriminant) = self
            .graph
            .syntax()
            .first_child_of_kind(variant, SyntaxNodeKind::EnumDiscriminant)
        else {
            return;
        };

        let Some(expression) = self.graph.syntax().child(discriminant, 0) else {
            return;
        };

        let Some(actual) = self.infer_expression_type(expression) else {
            return;
        };

        if self.is_assignable(base_type, actual)
            || self.is_integer_literal_compatible_with_enum_base(expression, base_type, actual)
        {
            return;
        }

        self.report_type_mismatch(expression, base_type, actual);
    }

    fn is_integer_literal_compatible_with_enum_base(
        &self,
        expression: NodeId,
        base_type: TypeId,
        actual: TypeId,
    ) -> bool {
        if !self.is_integer_type(base_type) || !self.is_integer_type(actual) {
            return false;
        }

        self.graph
            .syntax()
            .node(expression)
            .is_some_and(|node| node.kind() == SyntaxNodeKind::IntegerLiteral)
    }
}
