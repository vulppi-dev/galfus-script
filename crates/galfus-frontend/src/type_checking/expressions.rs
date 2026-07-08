use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{PrimitiveType, SymbolKind, SyntaxNodeKind, TypeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        self.infer_expression_type_with_expected(node, None)
    }

    pub(super) fn infer_expression_type_with_expected(
        &mut self,
        node: NodeId,
        expected: Option<TypeId>,
    ) -> Option<TypeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        if let Some(existing) = self.layer.node_type(node) {
            match syntax_node.kind() {
                SyntaxNodeKind::IntegerLiteral => {
                    if let Some(expected) = self.expected_integer_literal_type(expected) {
                        self.layer.bind_node_type(node, expected);
                        return Some(expected);
                    }
                }
                SyntaxNodeKind::FloatLiteral => {
                    if let Some(expected) = self.expected_float_literal_type(expected) {
                        self.layer.bind_node_type(node, expected);
                        return Some(expected);
                    }
                }
                _ => {}
            }

            return Some(existing);
        }

        let ty = match syntax_node.kind() {
            SyntaxNodeKind::IntegerLiteral => Some(
                self.expected_integer_literal_type(expected)
                    .unwrap_or_else(|| self.layer.table().primitive(PrimitiveType::Int32)),
            ),

            SyntaxNodeKind::FloatLiteral => Some(
                self.expected_float_literal_type(expected)
                    .unwrap_or_else(|| self.layer.table().primitive(PrimitiveType::Float32)),
            ),

            SyntaxNodeKind::BoolLiteral => Some(self.layer.table().primitive(PrimitiveType::Bool)),

            SyntaxNodeKind::NullLiteral => Some(self.layer.table().primitive(PrimitiveType::Null)),

            SyntaxNodeKind::StringLiteral => self.infer_string_literal_type(node),

            SyntaxNodeKind::ArrayLiteral => self.infer_array_literal_type(node, expected),

            SyntaxNodeKind::StructLiteral => self.infer_struct_literal_type(node),

            SyntaxNodeKind::InferredStructLiteral => {
                self.report_cannot_infer_type(
                    node,
                    "inferred struct literal requires an expected struct type",
                );

                Some(self.layer.table_mut().error())
            }

            SyntaxNodeKind::GroupedExpression => {
                let inner = self.graph.syntax().child(node, 0)?;
                self.infer_expression_type_with_expected(inner, expected)
            }

            SyntaxNodeKind::WildcardExpression => {
                self.report_cannot_infer_type(node, "wildcard cannot be used as a value");
                let ty = self.layer.table_mut().error();
                self.layer.bind_node_type(node, ty);
                Some(ty)
            }

            SyntaxNodeKind::NameExpression => self.infer_name_expression_type(node),

            SyntaxNodeKind::PathExpression => self.infer_path_variant_expression_type(node),

            SyntaxNodeKind::GenericExpression => self.infer_generic_expression_type(node),

            SyntaxNodeKind::CallExpression => self.infer_call_expression_type(node, expected),

            SyntaxNodeKind::ArrowFunctionExpression => {
                self.infer_arrow_function_expression_type(node)
            }

            SyntaxNodeKind::MatchExpression => self.infer_match_expression_type(node),

            SyntaxNodeKind::InstanceofExpression => {
                self.infer_instanceof_expression_type(node, expected)
            }

            SyntaxNodeKind::MemberExpression => self.infer_member_expression_type(node, false),

            SyntaxNodeKind::NullSafeMemberExpression => {
                self.infer_member_expression_type(node, true)
            }

            SyntaxNodeKind::IndexExpression => self.infer_index_expression_type(node),

            SyntaxNodeKind::BinaryExpression => self.infer_binary_expression_type(node, expected),

            SyntaxNodeKind::UnaryExpression => self.infer_unary_expression_type(node),

            SyntaxNodeKind::RangeExpression => self.infer_range_expression_type(node),

            SyntaxNodeKind::TupleExpression => self.infer_tuple_expression_type(node, expected),

            SyntaxNodeKind::CastExpression => self.infer_cast_expression_type(node),

            SyntaxNodeKind::CopyExpression => {
                let value = self.graph.syntax().child(node, 0)?;
                let ty = self.infer_expression_type_with_expected(value, expected)?;
                if self.is_fieldless_struct_type(ty) {
                    self.report_invalid_copy_target(node, ty);
                }
                Some(ty)
            }

            SyntaxNodeKind::NewArrayExpression => self.infer_new_array_expression_type(node),

            _ => None,
        }?;

        let ty = self.apply_active_type_substitutions(ty);

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    fn infer_name_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let resolution = self.graph.resolution()?;

        let symbol = resolution.reference_symbol(node).or_else(|| {
            let identifier = self
                .graph
                .syntax()
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)?;

            resolution.reference_symbol(identifier)
        })?;

        self.layer
            .symbol_type(symbol)
            .or_else(|| {
                if let Some(symbol_data) = resolution.symbol(symbol) {
                    if symbol_data.kind() == SymbolKind::ImportNamespace {
                        let ty = self.layer.table_mut().intern(TypeKind::Named { symbol });
                        self.layer.bind_symbol_type(symbol, ty);
                        return Some(ty);
                    }
                }
                self.infer_unbound_symbol_type(symbol)
            })
            .map(|ty| self.apply_active_type_substitutions(ty))
    }

    fn expected_integer_literal_type(&self, expected: Option<TypeId>) -> Option<TypeId> {
        let expected = self.resolve_alias_type(expected?);

        match self.layer.table().kind(expected) {
            Some(TypeKind::Primitive(
                PrimitiveType::Int8
                | PrimitiveType::Int16
                | PrimitiveType::Int32
                | PrimitiveType::Int64
                | PrimitiveType::Uint8
                | PrimitiveType::Uint16
                | PrimitiveType::Uint32
                | PrimitiveType::Uint64
                | PrimitiveType::Float16
                | PrimitiveType::Float32
                | PrimitiveType::Float64,
            )) => Some(expected),
            _ => None,
        }
    }

    fn expected_float_literal_type(&self, expected: Option<TypeId>) -> Option<TypeId> {
        let expected = self.resolve_alias_type(expected?);

        match self.layer.table().kind(expected) {
            Some(TypeKind::Primitive(
                PrimitiveType::Float16 | PrimitiveType::Float32 | PrimitiveType::Float64,
            )) => Some(expected),
            _ => None,
        }
    }

    pub(super) fn infer_unbound_symbol_type(&mut self, symbol: SymbolId) -> Option<TypeId> {
        let root = self.graph.syntax().root()?;
        let initializer = self.find_initializer_for_symbol(root, symbol)?;

        let error = self.layer.table_mut().error();
        self.layer.bind_symbol_type(symbol, error);

        let ty = self.infer_expression_type(initializer)?;
        self.layer.bind_symbol_type(symbol, ty);

        Some(ty)
    }

    fn find_initializer_for_symbol(&self, node: NodeId, symbol: SymbolId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        if matches!(
            syntax_node.kind(),
            SyntaxNodeKind::VarItem
                | SyntaxNodeKind::ConstItem
                | SyntaxNodeKind::VarStatement
                | SyntaxNodeKind::ConstStatement
        ) {
            let symbols = self.declaration_symbols_in_node(
                node,
                &[
                    SymbolKind::Var,
                    SymbolKind::Const,
                    SymbolKind::PatternBinding,
                    SymbolKind::TypePatternBinding,
                ],
            );

            if symbols.contains(&symbol) {
                let initializer = self
                    .graph
                    .syntax()
                    .first_child_of_kind(node, SyntaxNodeKind::Initializer)?;

                return self.graph.syntax().child(initializer, 0);
            }
        }

        for child in syntax_node.children() {
            if let Some(initializer) = self.find_initializer_for_symbol(*child, symbol) {
                return Some(initializer);
            }
        }

        None
    }

    fn infer_tuple_expression_type(
        &mut self,
        node: NodeId,
        expected: Option<TypeId>,
    ) -> Option<TypeId> {
        let expected_elements = expected.and_then(|expected_ty| {
            let resolved = self.resolve_alias_type(expected_ty);
            match self.layer.table().kind(resolved) {
                Some(TypeKind::Tuple { elements }) => Some(elements.clone()),
                _ => None,
            }
        });

        let children = self.graph.syntax().node(node)?.children().to_vec();
        let mut elements = Vec::with_capacity(children.len());

        for (index, child) in children.into_iter().enumerate() {
            let expected_element = expected_elements
                .as_ref()
                .and_then(|elements| elements.get(index))
                .copied();
            let ty = self.infer_expression_type_with_expected(child, expected_element)?;
            elements.push(ty);
        }

        let ty = self.layer.table_mut().intern_tuple(elements);
        self.layer.bind_node_type(node, ty);

        Some(ty)
    }

    fn infer_cast_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let type_node = self.first_type_child(node)?;
        let target_type = self.layer.node_type(type_node)?;
        if let Some(val_node) = self.graph.syntax().child(node, 1) {
            self.infer_expression_type(val_node);
        }
        Some(target_type)
    }

    fn infer_new_array_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        // child 0 is the type node ([T] or [T; N]); child 1 (if present) is the
        // storage identifier — not relevant for the type of the expression.
        let type_node = self.graph.syntax().child(node, 0)?;
        // The type resolver processes all type nodes in the graph before expression
        // inference runs, so node_type should already be populated.
        // Fall back to first_type_child for safety (e.g. if child 0 is a wrapper).
        self.layer.node_type(type_node).or_else(|| {
            self.first_type_child(node)
                .and_then(|n| self.layer.node_type(n))
        })
    }
}
