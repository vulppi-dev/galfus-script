use galfus_core::{NodeId, TypeId};

use crate::{PrimitiveType, TypeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_binary_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let left = self.graph.syntax().child(node, 0)?;
        let operator = self.graph.syntax().child(node, 1)?;
        let right = self.graph.syntax().child(node, 2)?;

        let left_type = self.infer_expression_type(left)?;
        let right_type = self.infer_expression_type(right)?;

        let operator_text = self.node_text(operator);

        let ty = match operator_text.as_str() {
            "+" | "-" | "*" | "/" | "%" | "**" => {
                self.check_numeric_binary_operator(operator, left_type, right_type)?
            }

            "<" | "<=" | ">" | ">=" => {
                self.check_numeric_comparison_operator(operator, left_type, right_type)?
            }

            "==" | "!=" => self.check_equality_operator(operator, left_type, right_type)?,

            "&&" | "||" => self.check_bool_binary_operator(operator, left_type, right_type)?,

            "&" | "|" | "^" => {
                self.check_integer_binary_operator(operator, left_type, right_type)?
            }

            "<<" | ">>" => self.check_shift_operator(operator, left_type, right_type)?,

            _ => {
                self.report_unsupported_operator(operator, operator_text.as_str());
                self.layer.table_mut().error()
            }
        };

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    pub(super) fn infer_unary_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let operator = self.graph.syntax().child(node, 0)?;
        let operand = self.graph.syntax().child(node, 1)?;

        let operand_type = self.infer_expression_type(operand)?;
        let operator_text = self.node_text(operator);

        let ty = match operator_text.as_str() {
            "+" | "-" => self.check_numeric_unary_operator(operator, operand_type)?,

            "!" => self.check_bool_unary_operator(operator, operand_type)?,

            "~" => self.check_integer_unary_operator(operator, operand_type)?,

            _ => {
                self.report_unsupported_operator(operator, operator_text.as_str());
                self.layer.table_mut().error()
            }
        };

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    fn check_numeric_binary_operator(
        &mut self,
        operator: NodeId,
        left: TypeId,
        right: TypeId,
    ) -> Option<TypeId> {
        if self.is_same_numeric_type(left, right) {
            return Some(left);
        }

        self.report_operator_type_error(operator, "numeric operands of the same type", left, right);
        Some(self.layer.table_mut().error())
    }

    fn check_numeric_comparison_operator(
        &mut self,
        operator: NodeId,
        left: TypeId,
        right: TypeId,
    ) -> Option<TypeId> {
        if self.is_same_numeric_type(left, right) {
            return Some(self.layer.table().primitive(PrimitiveType::Bool));
        }

        self.report_operator_type_error(operator, "numeric operands of the same type", left, right);
        Some(self.layer.table_mut().error())
    }

    fn check_equality_operator(
        &mut self,
        operator: NodeId,
        left: TypeId,
        right: TypeId,
    ) -> Option<TypeId> {
        if left == right || self.is_assignable(left, right) || self.is_assignable(right, left) {
            return Some(self.layer.table().primitive(PrimitiveType::Bool));
        }

        self.report_operator_type_error(operator, "comparable operands", left, right);
        Some(self.layer.table_mut().error())
    }

    fn check_bool_binary_operator(
        &mut self,
        operator: NodeId,
        left: TypeId,
        right: TypeId,
    ) -> Option<TypeId> {
        let bool_type = self.layer.table().primitive(PrimitiveType::Bool);

        if left == bool_type && right == bool_type {
            return Some(bool_type);
        }

        self.report_operator_type_error(operator, "bool operands", left, right);
        Some(self.layer.table_mut().error())
    }

    fn check_integer_binary_operator(
        &mut self,
        operator: NodeId,
        left: TypeId,
        right: TypeId,
    ) -> Option<TypeId> {
        if self.is_same_integer_type(left, right) {
            return Some(left);
        }

        self.report_operator_type_error(operator, "integer operands of the same type", left, right);
        Some(self.layer.table_mut().error())
    }

    fn check_shift_operator(
        &mut self,
        operator: NodeId,
        left: TypeId,
        right: TypeId,
    ) -> Option<TypeId> {
        if self.is_integer_type(left) && self.is_integer_type(right) {
            return Some(left);
        }

        self.report_operator_type_error(operator, "integer operands", left, right);
        Some(self.layer.table_mut().error())
    }

    fn check_numeric_unary_operator(
        &mut self,
        operator: NodeId,
        operand: TypeId,
    ) -> Option<TypeId> {
        if self.is_numeric_type(operand) {
            return Some(operand);
        }

        self.report_unary_operator_type_error(operator, "numeric operand", operand);
        Some(self.layer.table_mut().error())
    }

    fn check_bool_unary_operator(&mut self, operator: NodeId, operand: TypeId) -> Option<TypeId> {
        let bool_type = self.layer.table().primitive(PrimitiveType::Bool);

        if operand == bool_type {
            return Some(bool_type);
        }

        self.report_unary_operator_type_error(operator, "bool operand", operand);
        Some(self.layer.table_mut().error())
    }

    fn check_integer_unary_operator(
        &mut self,
        operator: NodeId,
        operand: TypeId,
    ) -> Option<TypeId> {
        if self.is_integer_type(operand) {
            return Some(operand);
        }

        self.report_unary_operator_type_error(operator, "integer operand", operand);
        Some(self.layer.table_mut().error())
    }

    pub(super) fn is_same_numeric_type(&self, left: TypeId, right: TypeId) -> bool {
        left == right && self.is_numeric_type(left)
    }

    pub(super) fn is_same_integer_type(&self, left: TypeId, right: TypeId) -> bool {
        left == right && self.is_integer_type(left)
    }

    pub(super) fn is_numeric_type(&self, ty: TypeId) -> bool {
        match self.layer.table().kind(ty) {
            Some(TypeKind::Primitive(primitive)) => self.is_numeric_primitive(*primitive),
            Some(TypeKind::Error) => true,
            _ => false,
        }
    }

    pub(super) fn is_integer_type(&self, ty: TypeId) -> bool {
        match self.layer.table().kind(ty) {
            Some(TypeKind::Primitive(primitive)) => self.is_integer_primitive(*primitive),
            Some(TypeKind::Error) => true,
            _ => false,
        }
    }

    fn is_numeric_primitive(&self, primitive: PrimitiveType) -> bool {
        matches!(
            primitive,
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
                | PrimitiveType::Float64
        )
    }

    fn is_integer_primitive(&self, primitive: PrimitiveType) -> bool {
        matches!(
            primitive,
            PrimitiveType::Int8
                | PrimitiveType::Int16
                | PrimitiveType::Int32
                | PrimitiveType::Int64
                | PrimitiveType::Uint8
                | PrimitiveType::Uint16
                | PrimitiveType::Uint32
                | PrimitiveType::Uint64
        )
    }
}
