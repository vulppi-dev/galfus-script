use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{PrimitiveType, SymbolKind, SyntaxNodeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        if let Some(existing) = self.layer.node_type(node) {
            return Some(existing);
        }

        let syntax_node = self.graph.syntax().node(node)?;

        let ty = match syntax_node.kind() {
            SyntaxNodeKind::IntegerLiteral => {
                Some(self.layer.table().primitive(PrimitiveType::Int32))
            }

            SyntaxNodeKind::FloatLiteral => {
                Some(self.layer.table().primitive(PrimitiveType::Float64))
            }

            SyntaxNodeKind::BoolLiteral => Some(self.layer.table().primitive(PrimitiveType::Bool)),

            SyntaxNodeKind::NullLiteral => Some(self.layer.table().primitive(PrimitiveType::Null)),

            SyntaxNodeKind::StringLiteral => self.infer_string_literal_type(node),

            SyntaxNodeKind::ArrayLiteral => self.infer_array_literal_type(node),

            SyntaxNodeKind::StructLiteral => self.infer_struct_literal_type(node),

            SyntaxNodeKind::GroupedExpression => {
                let inner = self.graph.syntax().child(node, 0)?;
                self.infer_expression_type(inner)
            }

            SyntaxNodeKind::NameExpression => self.infer_name_expression_type(node),

            SyntaxNodeKind::PathExpression => self.infer_path_variant_expression_type(node),

            SyntaxNodeKind::CallExpression => self.infer_call_expression_type(node),

            SyntaxNodeKind::MatchExpression => self.infer_match_expression_type(node),

            SyntaxNodeKind::InstanceofExpression => self.infer_instanceof_expression_type(node),

            SyntaxNodeKind::MemberExpression => self.infer_member_expression_type(node, false),

            SyntaxNodeKind::NullSafeMemberExpression => {
                self.infer_member_expression_type(node, true)
            }

            SyntaxNodeKind::IndexExpression => self.infer_index_expression_type(node),

            SyntaxNodeKind::BinaryExpression => self.infer_binary_expression_type(node),

            SyntaxNodeKind::UnaryExpression => self.infer_unary_expression_type(node),

            SyntaxNodeKind::RangeExpression => self.infer_range_expression_type(node),

            SyntaxNodeKind::TupleExpression => self.infer_tuple_expression_type(node),

            SyntaxNodeKind::CastExpression => self.infer_cast_expression_type(node),

            SyntaxNodeKind::CopyExpression => {
                let value = self.graph.syntax().child(node, 0)?;
                self.infer_expression_type(value)
            }

            _ => None,
        }?;

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
            .or_else(|| self.infer_unbound_symbol_type(symbol))
    }

    fn infer_unbound_symbol_type(&mut self, symbol: SymbolId) -> Option<TypeId> {
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

    fn infer_tuple_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let elements = self
            .graph
            .syntax()
            .node(node)?
            .children()
            .to_vec()
            .into_iter()
            .map(|child| self.infer_expression_type(child))
            .collect::<Option<Vec<_>>>()?;

        let ty = self.layer.table_mut().intern_tuple(elements);
        self.layer.bind_node_type(node, ty);

        Some(ty)
    }

    fn infer_cast_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let type_node = self.first_type_child(node)?;
        self.layer.node_type(type_node)
    }
}
