use super::function::FunctionBuilder;
use crate::mir::*;
use galfus_core::{NodeId, StorageMetadata, TypeId};
use galfus_frontend::{SyntaxNode, SyntaxNodeKind, TypeKind};

impl<'b, 'a> FunctionBuilder<'b, 'a> {
    pub(super) fn lower_struct_literal(
        &mut self,
        expr_id: NodeId,
        node: &SyntaxNode,
        statements: &mut Vec<MirBody>,
    ) -> Operand {
        let syntax = self.builder.graph.syntax();
        let struct_type = self
            .builder
            .type_result
            .layer()
            .node_type(expr_id)
            .unwrap_or_else(|| TypeId::new(0));

        if let Some(struct_symbol) = self.struct_symbol_for_type(struct_type) {
            let fields_list_node = if node.kind() == SyntaxNodeKind::StructLiteral {
                node.child(1)
            } else {
                node.child(0)
            };

            let mut field_values = std::collections::HashMap::new();
            let mut spread_operands = Vec::new();

            let field_children = fields_list_node
                .and_then(|list_id| syntax.node(list_id))
                .map(|n| n.children())
                .unwrap_or(&[]);

            for &child_id in field_children {
                if let Some(child_node) = syntax.node(child_id) {
                    match child_node.kind() {
                        SyntaxNodeKind::StructLiteralField => {
                            let name_ident = syntax
                                .first_child_of_kind(child_id, SyntaxNodeKind::Identifier)
                                .unwrap();
                            let name = self.builder.node_text(name_ident).to_string();
                            let val_expr = child_node.child(1).unwrap();
                            let op = self.lower_expression(val_expr, statements);
                            field_values.insert(name, op);
                        }
                        SyntaxNodeKind::StructLiteralFieldShorthand => {
                            let name_ident = child_node.first_child().unwrap();
                            let name = self.builder.node_text(name_ident).to_string();
                            let op = self.lower_expression(name_ident, statements);
                            field_values.insert(name, op);
                        }
                        SyntaxNodeKind::SpreadStructLiteralField => {
                            let spread_expr = child_node.child(0).unwrap();
                            let op = self.lower_expression(spread_expr, statements);
                            spread_operands.push((spread_expr, op));
                        }
                        _ => {}
                    }
                }
            }

            let struct_fields_decl = self.get_struct_fields(struct_symbol);
            let mut fields = Vec::new();

            for (field_name, field_ty) in struct_fields_decl {
                if let Some(op) = field_values.remove(&field_name) {
                    fields.push(op);
                } else {
                    // Try to get from spread
                    let mut found_in_spread = false;
                    for &(spread_expr, ref spread_op) in &spread_operands {
                        let spread_ty = self
                            .builder
                            .type_result
                            .layer()
                            .node_type(spread_expr)
                            .unwrap_or_else(|| TypeId::new(0));
                        if let Some(spread_sym) = self.struct_symbol_for_type(spread_ty) {
                            let spread_fields = self.get_struct_fields(spread_sym);
                            if spread_fields.iter().any(|(n, _)| *n == field_name) {
                                let temp_id = self.declare_local(None, field_ty);
                                self.current_instructions.push(Instruction::Assign(
                                    temp_id,
                                    RValue::MemberAccess(spread_op.clone(), field_name.clone()),
                                ));
                                fields.push(Operand::Local(temp_id));
                                found_in_spread = true;
                                break;
                            }
                        }
                    }

                    if !found_in_spread {
                        // Try default value
                        if let Some(default_expr) =
                            self.find_struct_field_default_expr(struct_symbol, &field_name)
                        {
                            let op = self.lower_expression(default_expr, statements);
                            fields.push(op);
                        } else {
                            fields.push(Operand::Constant(Constant::Null));
                        }
                    }
                }
            }

            let temp_id = self.declare_local(None, struct_type);
            self.current_instructions.push(Instruction::Assign(
                temp_id,
                RValue::NewStruct {
                    struct_type,
                    fields,
                    storage_meta: StorageMetadata::Local,
                },
            ));
            Operand::Local(temp_id)
        } else {
            Operand::Constant(Constant::Null)
        }
    }

    pub(super) fn lower_array_literal(
        &mut self,
        expr_id: NodeId,
        node: &SyntaxNode,
        statements: &mut Vec<MirBody>,
    ) -> Operand {
        let syntax = self.builder.graph.syntax();
        let array_type = self
            .builder
            .type_result
            .layer()
            .node_type(expr_id)
            .unwrap_or_else(|| TypeId::new(0));

        let resolved_array_type = self.builder.resolve_alias_type(array_type);
        let expected_element_type = match self
            .builder
            .type_result
            .layer()
            .table()
            .kind(resolved_array_type)
        {
            Some(TypeKind::Array { element }) => Some(*element),
            Some(TypeKind::FixedArray { element, .. }) => Some(*element),
            _ => None,
        };

        let has_spread = node.children().iter().any(|&child_id| {
            syntax
                .node(child_id)
                .is_some_and(|child_node| child_node.kind() == SyntaxNodeKind::SpreadArrayElement)
        });

        if has_spread {
            let mut elements = Vec::new();
            for &child_id in node.children() {
                if let Some(child_node) = syntax.node(child_id) {
                    match child_node.kind() {
                        SyntaxNodeKind::ArrayElement => {
                            let val_expr = child_node.child(0).unwrap();
                            let val_expr_ty = self
                                .builder
                                .type_result
                                .layer()
                                .node_type(val_expr)
                                .unwrap_or_else(|| TypeId::new(0));
                            let op = self.lower_expression(val_expr, statements);
                            let casted_op = if let Some(elem_ty) = expected_element_type {
                                self.insert_cast_if_needed(op, val_expr_ty, elem_ty)
                            } else {
                                op
                            };
                            elements.push(ArrayLiteralElement::Single(casted_op));
                        }
                        SyntaxNodeKind::SpreadArrayElement => {
                            let spread_expr = child_node.child(0).unwrap();
                            let op = self.lower_expression(spread_expr, statements);
                            elements.push(ArrayLiteralElement::Spread(op));
                        }
                        _ => {
                            let val_expr_ty = self
                                .builder
                                .type_result
                                .layer()
                                .node_type(child_id)
                                .unwrap_or_else(|| TypeId::new(0));
                            let op = self.lower_expression(child_id, statements);
                            let casted_op = if let Some(elem_ty) = expected_element_type {
                                self.insert_cast_if_needed(op, val_expr_ty, elem_ty)
                            } else {
                                op
                            };
                            elements.push(ArrayLiteralElement::Single(casted_op));
                        }
                    }
                }
            }

            let temp_id = self.declare_local(None, array_type);
            self.current_instructions.push(Instruction::Assign(
                temp_id,
                RValue::NewArrayDynamic(array_type, elements),
            ));
            Operand::Local(temp_id)
        } else {
            let mut elements = Vec::new();
            for &child_id in node.children() {
                if let Some(child_node) = syntax.node(child_id) {
                    match child_node.kind() {
                        SyntaxNodeKind::ArrayElement => {
                            let val_expr = child_node.child(0).unwrap();
                            let val_expr_ty = self
                                .builder
                                .type_result
                                .layer()
                                .node_type(val_expr)
                                .unwrap_or_else(|| TypeId::new(0));
                            let op = self.lower_expression(val_expr, statements);
                            let casted_op = if let Some(elem_ty) = expected_element_type {
                                self.insert_cast_if_needed(op, val_expr_ty, elem_ty)
                            } else {
                                op
                            };
                            elements.push(casted_op);
                        }
                        _ => {
                            let val_expr_ty = self
                                .builder
                                .type_result
                                .layer()
                                .node_type(child_id)
                                .unwrap_or_else(|| TypeId::new(0));
                            let op = self.lower_expression(child_id, statements);
                            let casted_op = if let Some(elem_ty) = expected_element_type {
                                self.insert_cast_if_needed(op, val_expr_ty, elem_ty)
                            } else {
                                op
                            };
                            elements.push(casted_op);
                        }
                    }
                }
            }

            let temp_id = self.declare_local(None, array_type);
            self.current_instructions.push(Instruction::Assign(
                temp_id,
                RValue::NewArray(array_type, elements),
            ));
            Operand::Local(temp_id)
        }
    }

    pub(super) fn lower_tuple_literal(
        &mut self,
        expr_id: NodeId,
        node: &SyntaxNode,
        statements: &mut Vec<MirBody>,
    ) -> Operand {
        let mut elements = Vec::new();
        let mut element_types = Vec::new();
        for &child in node.children() {
            let operand = self.lower_expression(child, statements);
            elements.push(operand);

            let ty = self
                .builder
                .type_result
                .layer()
                .node_type(child)
                .unwrap_or_else(|| TypeId::new(0));
            element_types.push(ty);
        }

        let ty = self
            .builder
            .type_result
            .layer()
            .node_type(expr_id)
            .unwrap_or_else(|| self.builder.find_tuple_type(&element_types));

        let temp_id = self.declare_local(None, ty);
        self.current_instructions
            .push(Instruction::Assign(temp_id, RValue::NewTuple(ty, elements)));
        Operand::Local(temp_id)
    }

    /// Lower `new([T; N])` / `new([T; N], shared)`.
    ///
    /// child 0 — `FixedArrayType` or `ArrayType` node  
    /// child 1 — optional storage-tag `Identifier` (e.g. `shared`), currently stored
    ///            in `StorageMetadata` but not yet acted upon by the VM.
    pub(super) fn lower_new_array_expression(
        &mut self,
        expr_id: NodeId,
        node: &SyntaxNode,
        _statements: &mut Vec<MirBody>,
    ) -> Operand {
        let type_layer = self.builder.type_result.layer();

        // The full array type (FixedArray TypeId) — used as the RValue's type tag.
        let array_type = type_layer
            .node_type(expr_id)
            .unwrap_or_else(|| TypeId::new(0));

        // Resolve element type and static size from FixedArrayType.
        let resolved = self.builder.resolve_alias_type(array_type);
        let (element_type, size) =
            match type_layer.table().kind(resolved) {
                Some(TypeKind::FixedArray { element, size: galfus_frontend::ArraySize::Known(n) }) => {
                    (*element, *n as usize)
                }
                Some(TypeKind::Array { element }) => {
                    // Dynamic array — size 0; caller must rely on runtime resize.
                    (*element, 0)
                }
                _ => return Operand::Constant(Constant::Null),
            };

        // Optional second child: storage tag identifier (e.g. `shared`).
        let storage = if node.child_count() > 1 {
            let storage_ident = node.child(1).unwrap();
            let tag = self.builder.node_text(storage_ident);
            if tag == "shared" {
                StorageMetadata::Shared
            } else {
                StorageMetadata::Local
            }
        } else {
            StorageMetadata::Local
        };

        let temp_id = self.declare_local(None, array_type);
        self.current_instructions.push(Instruction::Assign(
            temp_id,
            RValue::NewArrayZeroed {
                array_type,
                element_type,
                size,
                storage,
            },
        ));
        Operand::Local(temp_id)
    }
}
