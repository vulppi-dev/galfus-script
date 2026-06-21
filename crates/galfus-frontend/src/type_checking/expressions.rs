use galfus_core::{NodeId, TypeId};

use crate::{PrimitiveType, SyntaxNodeKind};

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

            SyntaxNodeKind::GroupedExpression => {
                let inner = self.graph.syntax().child(node, 0)?;
                self.infer_expression_type(inner)
            }

            SyntaxNodeKind::NameExpression => self.infer_name_expression_type(node),

            SyntaxNodeKind::CallExpression => self.infer_call_expression_type(node),

            SyntaxNodeKind::BinaryExpression => self.infer_binary_expression_type(node),

            SyntaxNodeKind::UnaryExpression => self.infer_unary_expression_type(node),

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

    fn infer_name_expression_type(&self, node: NodeId) -> Option<TypeId> {
        let resolution = self.graph.resolution()?;

        let symbol = resolution.reference_symbol(node).or_else(|| {
            let identifier = self
                .graph
                .syntax()
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)?;

            resolution.reference_symbol(identifier)
        })?;

        self.layer.symbol_type(symbol)
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

        Some(self.layer.table_mut().intern_tuple(elements))
    }

    fn infer_cast_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let type_node = self.first_type_child(node)?;
        self.layer.node_type(type_node)
    }
}
