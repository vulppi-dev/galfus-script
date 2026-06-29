use super::function::FunctionBuilder;
use super::*;

impl<'b, 'a> FunctionBuilder<'b, 'a> {
    pub(super) fn lower_binary_op(&self, op_node_id: NodeId) -> MirBinaryOp {
        let syntax = self.builder.graph.syntax();
        if let Some(op) = syntax
            .node(op_node_id)
            .and_then(|node| node.binary_operator())
        {
            return match op {
                galfus_frontend::BinaryOperatorKind::Add => MirBinaryOp::Add,
                galfus_frontend::BinaryOperatorKind::Subtract => MirBinaryOp::Subtract,
                galfus_frontend::BinaryOperatorKind::Multiply => MirBinaryOp::Multiply,
                galfus_frontend::BinaryOperatorKind::Divide => MirBinaryOp::Divide,
                galfus_frontend::BinaryOperatorKind::Remainder => MirBinaryOp::Remainder,
                galfus_frontend::BinaryOperatorKind::Power => MirBinaryOp::Power,
                galfus_frontend::BinaryOperatorKind::ShiftLeft => MirBinaryOp::ShiftLeft,
                galfus_frontend::BinaryOperatorKind::ShiftRight => MirBinaryOp::ShiftRight,
                galfus_frontend::BinaryOperatorKind::BitwiseAnd => MirBinaryOp::BitwiseAnd,
                galfus_frontend::BinaryOperatorKind::BitwiseOr => MirBinaryOp::BitwiseOr,
                galfus_frontend::BinaryOperatorKind::BitwiseXor => MirBinaryOp::BitwiseXor,
                galfus_frontend::BinaryOperatorKind::Equal => MirBinaryOp::Equal,
                galfus_frontend::BinaryOperatorKind::NotEqual => MirBinaryOp::NotEqual,
                galfus_frontend::BinaryOperatorKind::Less => MirBinaryOp::Less,
                galfus_frontend::BinaryOperatorKind::LessEqual => MirBinaryOp::LessEqual,
                galfus_frontend::BinaryOperatorKind::Greater => MirBinaryOp::Greater,
                galfus_frontend::BinaryOperatorKind::GreaterEqual => MirBinaryOp::GreaterEqual,
                galfus_frontend::BinaryOperatorKind::LogicalAnd => MirBinaryOp::LogicalAnd,
                galfus_frontend::BinaryOperatorKind::LogicalOr => MirBinaryOp::LogicalOr,
                galfus_frontend::BinaryOperatorKind::NullFallback => MirBinaryOp::NullFallback,
            };
        }
        MirBinaryOp::Add
    }

    pub(super) fn lower_unary_op(&self, op_node_id: NodeId) -> MirUnaryOp {
        let syntax = self.builder.graph.syntax();
        if let Some(op) = syntax
            .node(op_node_id)
            .and_then(|node| node.unary_operator())
        {
            return match op {
                galfus_frontend::UnaryOperatorKind::Negate => MirUnaryOp::Negate,
                galfus_frontend::UnaryOperatorKind::Not => MirUnaryOp::Not,
                galfus_frontend::UnaryOperatorKind::BitwiseNot => MirUnaryOp::BitwiseNot,
            };
        }
        MirUnaryOp::Negate
    }

    pub(super) fn struct_symbol_for_type(&self, ty: TypeId) -> Option<SymbolId> {
        self.builder.struct_symbol_for_type(ty)
    }

    pub(super) fn find_struct_field_default_expr(
        &self,
        struct_symbol: SymbolId,
        field_name: &str,
    ) -> Option<NodeId> {
        self.builder
            .find_struct_field_default_expr(struct_symbol, field_name)
    }

    pub(super) fn get_struct_fields(&self, struct_symbol: SymbolId) -> Vec<(String, TypeId)> {
        self.builder.get_struct_fields(struct_symbol)
    }

    pub(super) fn find_tuple_type(&self, elements: &[TypeId]) -> TypeId {
        self.builder.find_tuple_type(elements)
    }
}

pub(super) fn parse_int(text: &str) -> Option<i64> {
    let clean = text.trim();
    if clean.starts_with("0x") || clean.starts_with("0X") {
        i64::from_str_radix(&clean[2..].replace('_', ""), 16).ok()
    } else if clean.starts_with("0o") || clean.starts_with("0O") {
        i64::from_str_radix(&clean[2..].replace('_', ""), 8).ok()
    } else if clean.starts_with("0b") || clean.starts_with("0B") {
        i64::from_str_radix(&clean[2..].replace('_', ""), 2).ok()
    } else {
        clean.replace('_', "").parse::<i64>().ok()
    }
}
