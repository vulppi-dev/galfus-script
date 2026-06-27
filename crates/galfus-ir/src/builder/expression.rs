use super::function::{FunctionBuilder, parse_int};
use crate::mir::*;
use galfus_core::{FunctionId, NodeId, StorageMetadata, TypeId};
use galfus_frontend::{PathReferenceKind, SyntaxNodeKind, TypeKind};

impl<'b, 'a> FunctionBuilder<'b, 'a> {
    pub(super) fn lower_expression(
        &mut self,
        expr_id: NodeId,
        statements: &mut Vec<MirBody>,
    ) -> Operand {
        let syntax = self.builder.graph.syntax();
        let Some(node) = syntax.node(expr_id) else {
            return Operand::Constant(Constant::Null);
        };
        let resolution = self.builder.graph.resolution();

        match node.kind() {
            SyntaxNodeKind::IntegerLiteral => {
                let text = self.builder.node_text(expr_id);
                let val = parse_int(text).unwrap_or(0);
                Operand::Constant(Constant::Int(val))
            }

            SyntaxNodeKind::FloatLiteral => {
                let text = self.builder.node_text(expr_id);
                let val = text.parse::<f64>().unwrap_or(0.0);
                Operand::Constant(Constant::Float(val))
            }

            SyntaxNodeKind::StringLiteral => {
                let text = self.builder.node_text(expr_id);
                let val = if (text.starts_with('"') && text.ends_with('"'))
                    || (text.starts_with('\'') && text.ends_with('\''))
                {
                    &text[1..text.len() - 1]
                } else {
                    text
                };
                Operand::Constant(Constant::String(val.to_string()))
            }

            SyntaxNodeKind::BoolLiteral => {
                let text = self.builder.node_text(expr_id);
                let val = text == "true";
                Operand::Constant(Constant::Bool(val))
            }

            SyntaxNodeKind::NullLiteral => Operand::Constant(Constant::Null),

            SyntaxNodeKind::NameExpression => {
                if let Some(res) = resolution {
                    let symbol = res.reference_symbol(expr_id).or_else(|| {
                        let ident =
                            syntax.first_child_of_kind(expr_id, SyntaxNodeKind::Identifier)?;
                        res.reference_symbol(ident)
                    });
                    if let Some(local_id) =
                        symbol.and_then(|sym| self.symbol_to_local.get(&sym).copied())
                    {
                        return Operand::Local(local_id);
                    }
                }
                Operand::Constant(Constant::Null)
            }

            SyntaxNodeKind::BinaryExpression => {
                let left = node.child(0).unwrap();
                let op_node = node.child(1).unwrap();
                let right = node.child(2).unwrap();

                let left_operand = self.lower_expression(left, statements);
                let right_operand = self.lower_expression(right, statements);

                let op = self.lower_binary_op(op_node);

                let ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(expr_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let temp_id = self.declare_local(None, ty);
                self.current_instructions.push(Instruction::Assign(
                    temp_id,
                    RValue::BinaryOp(op, left_operand, right_operand),
                ));
                Operand::Local(temp_id)
            }

            SyntaxNodeKind::UnaryExpression => {
                let op_node = node.child(0).unwrap();
                let operand_node = node.child(1).unwrap();

                let operand = self.lower_expression(operand_node, statements);
                let op = self.lower_unary_op(op_node);

                let ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(expr_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let temp_id = self.declare_local(None, ty);
                self.current_instructions
                    .push(Instruction::Assign(temp_id, RValue::UnaryOp(op, operand)));
                Operand::Local(temp_id)
            }

            SyntaxNodeKind::CastExpression => {
                let val_node = node.child(1).unwrap();
                let operand = self.lower_expression(val_node, statements);

                let ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(expr_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let temp_id = self.declare_local(None, ty);
                self.current_instructions
                    .push(Instruction::Assign(temp_id, RValue::Cast(operand, ty)));
                Operand::Local(temp_id)
            }

            SyntaxNodeKind::GroupedExpression => {
                if let Some(inner) = node.first_child() {
                    self.lower_expression(inner, statements)
                } else {
                    Operand::Constant(Constant::Null)
                }
            }

            SyntaxNodeKind::CallExpression => {
                let target_node = node.child(0).unwrap();
                let arg_list_node = node.child(1).unwrap();

                let mut args = Vec::new();
                let mut arg_types = Vec::new();
                if let Some(arg_list) = syntax.node(arg_list_node) {
                    for &arg_id in arg_list.children() {
                        let arg_expr = syntax
                            .node(arg_id)
                            .and_then(|n| {
                                if n.kind() == SyntaxNodeKind::Argument {
                                    syntax.child(arg_id, 0)
                                } else {
                                    Some(arg_id)
                                }
                            })
                            .unwrap_or(arg_id);

                        let arg_op = self.lower_expression(arg_expr, statements);
                        args.push(arg_op);

                        let arg_ty = self
                            .builder
                            .type_result
                            .layer()
                            .node_type(arg_expr)
                            .unwrap_or_else(|| TypeId::new(0));
                        arg_types.push(arg_ty);
                    }
                }

                // Check if it's a choice variant constructor call!
                let payload = if self.is_choice_variant_call_target(target_node) {
                    self.get_choice_variant_payload(target_node)
                } else {
                    None
                };
                if let Some((variant_name, owner_type, _payload_types)) = payload {
                    let payload_op = if args.is_empty() {
                        None
                    } else if args.len() == 1 {
                        Some(args[0].clone())
                    } else {
                        // Multiple arguments => build a tuple
                        let tuple_type = self.builder.find_tuple_type(&arg_types);
                        let tuple_temp = self.declare_local(None, tuple_type);
                        self.current_instructions.push(Instruction::Assign(
                            tuple_temp,
                            RValue::NewTuple(tuple_type, args),
                        ));
                        Some(Operand::Local(tuple_temp))
                    };

                    let choice_temp = self.declare_local(None, owner_type);
                    self.current_instructions.push(Instruction::Assign(
                        choice_temp,
                        RValue::Choice(owner_type, variant_name, payload_op),
                    ));
                    return Operand::Local(choice_temp);
                }

                let symbol = resolution.and_then(|res| {
                    res.reference_symbol(target_node).or_else(|| {
                        let ident =
                            syntax.first_child_of_kind(target_node, SyntaxNodeKind::Identifier)?;
                        res.reference_symbol(ident)
                    })
                });
                let func_id = symbol
                    .map(|sym| FunctionId::new(sym.raw()))
                    .unwrap_or_else(|| FunctionId::new(0));

                let ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(expr_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let temp_id = self.declare_local(None, ty);

                let instructions = std::mem::take(&mut self.current_instructions);
                let block_id = self.builder.next_block();
                statements.push(MirBody::BasicBlock(BasicBlock {
                    id: block_id,
                    instructions,
                    terminator: Terminator::Call {
                        func: func_id,
                        args,
                        destination: temp_id,
                    },
                }));

                Operand::Local(temp_id)
            }
            SyntaxNodeKind::PathExpression => {
                let kind = resolution.and_then(|res| res.path_reference_kind(expr_id));
                if let Some(kind) = kind {
                    match kind {
                        PathReferenceKind::EnumVariant => {
                            if let Some(variant_symbol) =
                                resolution.and_then(|res| res.path_reference_symbol(expr_id))
                            {
                                let val = self.get_enum_variant_value(variant_symbol);
                                return Operand::Constant(Constant::Int(val));
                            }
                        }
                        PathReferenceKind::ChoiceVariant => {
                            if let Some((variant_name, owner_type, _payload_types)) =
                                self.get_choice_variant_payload(expr_id)
                            {
                                let choice_temp = self.declare_local(None, owner_type);
                                self.current_instructions.push(Instruction::Assign(
                                    choice_temp,
                                    RValue::Choice(owner_type, variant_name, None),
                                ));
                                return Operand::Local(choice_temp);
                            }
                        }
                        _ => {}
                    }
                }
                Operand::Constant(Constant::Null)
            }

            SyntaxNodeKind::StructLiteral | SyntaxNodeKind::InferredStructLiteral => {
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
                                            RValue::MemberAccess(
                                                spread_op.clone(),
                                                field_name.clone(),
                                            ),
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
                    return Operand::Local(temp_id);
                }

                Operand::Constant(Constant::Null)
            }

            SyntaxNodeKind::ArrayLiteral => {
                let array_type = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(expr_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let mut elements = Vec::new();
                for &child_id in node.children() {
                    if let Some(child_node) = syntax.node(child_id) {
                        match child_node.kind() {
                            SyntaxNodeKind::ArrayElement => {
                                let val_expr = child_node.child(0).unwrap();
                                let op = self.lower_expression(val_expr, statements);
                                elements.push(op);
                            }
                            SyntaxNodeKind::SpreadArrayElement => {
                                let spread_expr = child_node.child(0).unwrap();
                                let op = self.lower_expression(spread_expr, statements);

                                let spread_ty = self
                                    .builder
                                    .type_result
                                    .layer()
                                    .node_type(spread_expr)
                                    .unwrap_or_else(|| TypeId::new(0));
                                let resolved = self.resolve_alias_type(spread_ty);

                                if let Some(TypeKind::FixedArray {
                                    element: element_ty,
                                    size: galfus_frontend::ArraySize::Known(len),
                                }) = self.builder.type_result.layer().table().kind(resolved)
                                {
                                    let len_val = *len;
                                    let elem_ty = *element_ty;
                                    for i in 0..len_val {
                                        let temp_id = self.declare_local(None, elem_ty);
                                        self.current_instructions.push(Instruction::Assign(
                                            temp_id,
                                            RValue::ArrayIndex(
                                                op.clone(),
                                                Operand::Constant(Constant::Int(i as i64)),
                                            ),
                                        ));
                                        elements.push(Operand::Local(temp_id));
                                    }
                                }
                            }
                            _ => {
                                let op = self.lower_expression(child_id, statements);
                                elements.push(op);
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

            SyntaxNodeKind::TupleExpression => {
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

            SyntaxNodeKind::MemberExpression | SyntaxNodeKind::NullSafeMemberExpression => {
                let obj_node = node.child(0).unwrap();
                let member_node = node.child(1).unwrap();
                let member_name = self.builder.node_text(member_node).to_string();

                let obj_operand = self.lower_expression(obj_node, statements);

                let ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(expr_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let temp_id = self.declare_local(None, ty);
                self.current_instructions.push(Instruction::Assign(
                    temp_id,
                    RValue::MemberAccess(obj_operand, member_name),
                ));
                Operand::Local(temp_id)
            }

            SyntaxNodeKind::IndexExpression => {
                let target_node = node.child(0).unwrap();
                let index_node = node.child(1).unwrap();

                let target_operand = self.lower_expression(target_node, statements);
                let index_operand = self.lower_expression(index_node, statements);

                let ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(expr_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let target_ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(target_node)
                    .unwrap_or_else(|| TypeId::new(0));

                let resolved_target = self.resolve_alias_type(target_ty);

                let is_tuple = matches!(
                    self.builder
                        .type_result
                        .layer()
                        .table()
                        .kind(resolved_target),
                    Some(TypeKind::Tuple { .. })
                );

                let temp_id = self.declare_local(None, ty);

                if is_tuple {
                    let index_str = match index_operand {
                        Operand::Constant(Constant::Int(val)) => val.to_string(),
                        _ => "0".to_string(),
                    };
                    self.current_instructions.push(Instruction::Assign(
                        temp_id,
                        RValue::MemberAccess(target_operand, index_str),
                    ));
                } else {
                    self.current_instructions.push(Instruction::Assign(
                        temp_id,
                        RValue::ArrayIndex(target_operand, index_operand),
                    ));
                }
                Operand::Local(temp_id)
            }

            SyntaxNodeKind::MatchExpression | SyntaxNodeKind::InstanceofExpression => {
                let subject_node = node.child(0).unwrap();
                let arms_node = node.child(1).unwrap();

                let subject_type = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(subject_node)
                    .unwrap_or_else(|| TypeId::new(0));

                let match_type = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(expr_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let subject_op = self.lower_expression(subject_node, statements);

                let subject_temp = self.declare_local(None, subject_type);
                self.current_instructions
                    .push(Instruction::Assign(subject_temp, RValue::Use(subject_op)));
                let subject_local_op = Operand::Local(subject_temp);

                let match_result = self.declare_local(None, match_type);

                self.flush_current_instructions(statements);

                let arms_syntax_node = syntax.node(arms_node).unwrap();
                let arm_nodes = arms_syntax_node.children().to_vec();

                let match_mir =
                    self.lower_match_arms(&arm_nodes, 0, &subject_local_op, match_result);
                statements.push(match_mir);

                Operand::Local(match_result)
            }

            _ => Operand::Constant(Constant::Null),
        }
    }
}
