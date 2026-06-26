use galfus_core::{NodeId, TypeId};

use crate::{ArraySize, PrimitiveType, SyntaxNodeKind, TypeKind};

use super::DeclarationTypeChecker;

#[derive(Debug, Clone, Copy)]
struct ArrayLiteralElementType {
    node: NodeId,
    ty: TypeId,
    len: u64,
    has_error: bool,
}

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_string_literal_type(&mut self, node: NodeId) -> Option<TypeId> {
        let uint8_type = self.layer.table().primitive(PrimitiveType::Uint8);
        let ty = self.layer.table_mut().intern_array(uint8_type);

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    pub(super) fn infer_array_literal_type(&mut self, node: NodeId) -> Option<TypeId> {
        let elements = self.graph.syntax().node(node)?.children().to_vec();

        if elements.is_empty() {
            self.report_empty_array_literal(node);

            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        }

        let mut element_types = Vec::new();

        for element in elements {
            let Some(element_type) = self.infer_array_literal_element_type(element) else {
                continue;
            };

            element_types.push(element_type);
        }

        let Some(expected_element_type) = element_types
            .iter()
            .find(|element| !element.has_error)
            .map(|element| element.ty)
        else {
            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        };

        let mut has_error = element_types.iter().any(|element| element.has_error);

        for element in element_types.iter().copied() {
            if element.has_error {
                continue;
            }

            if self.is_assignable(expected_element_type, element.ty) {
                continue;
            }

            self.report_type_mismatch(element.node, expected_element_type, element.ty);
            has_error = true;
        }

        let total_len = element_types.iter().map(|element| element.len).sum::<u64>();

        if total_len == 0 {
            self.report_empty_array_literal(node);

            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        }

        let ty = if has_error {
            self.layer.table_mut().error()
        } else {
            self.layer
                .table_mut()
                .intern_fixed_array(expected_element_type, ArraySize::Known(total_len))
        };

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    fn infer_array_literal_element_type(
        &mut self,
        element: NodeId,
    ) -> Option<ArrayLiteralElementType> {
        let element_node = self.graph.syntax().node(element)?;

        match element_node.kind() {
            SyntaxNodeKind::ArrayElement => {
                let expression = self.graph.syntax().child(element, 0)?;
                let ty = self.infer_expression_type(expression)?;

                self.layer.bind_node_type(element, ty);

                Some(ArrayLiteralElementType {
                    node: expression,
                    ty,
                    len: 1,
                    has_error: false,
                })
            }

            SyntaxNodeKind::SpreadArrayElement => {
                let expression = self.graph.syntax().child(element, 0)?;
                let spread_type = self.infer_expression_type(expression)?;

                let result = self.array_literal_spread_element_type(expression, spread_type)?;

                self.layer.bind_node_type(element, result.ty);

                Some(result)
            }

            _ => {
                let ty = self.infer_expression_type(element)?;

                Some(ArrayLiteralElementType {
                    node: element,
                    ty,
                    len: 1,
                    has_error: false,
                })
            }
        }
    }

    fn array_literal_spread_element_type(
        &mut self,
        expression: NodeId,
        spread_type: TypeId,
    ) -> Option<ArrayLiteralElementType> {
        let resolved = self.resolve_alias_type(spread_type);

        match self.layer.table().kind(resolved).cloned() {
            Some(TypeKind::FixedArray {
                element,
                size: ArraySize::Known(len),
            }) => Some(ArrayLiteralElementType {
                node: expression,
                ty: element,
                len,
                has_error: false,
            }),

            Some(TypeKind::FixedArray { .. }) | Some(TypeKind::Array { .. }) => {
                self.report_dynamic_spread_in_array_literal(expression, spread_type);

                let error = self.layer.table_mut().error();

                Some(ArrayLiteralElementType {
                    node: expression,
                    ty: error,
                    len: 0,
                    has_error: true,
                })
            }

            Some(TypeKind::Error) => Some(ArrayLiteralElementType {
                node: expression,
                ty: resolved,
                len: 0,
                has_error: true,
            }),

            _ => {
                self.report_invalid_spread_target(expression, spread_type);

                let error = self.layer.table_mut().error();

                Some(ArrayLiteralElementType {
                    node: expression,
                    ty: error,
                    len: 0,
                    has_error: true,
                })
            }
        }
    }
}
