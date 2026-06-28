use super::function::{FunctionBuilder, parse_int};
use crate::mir::*;
use galfus_core::{FunctionId, NodeId, TypeId};
use galfus_frontend::{PathReferenceKind, SymbolKind, SyntaxNodeKind, TypeKind};

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
                    if let Some(sym) = symbol {
                        if let Some(local_id) = self.symbol_to_local.get(&sym).copied() {
                            return Operand::Local(local_id);
                        } else {
                            let is_global = matches!(
                                res.symbol(sym).map(|s| s.kind()),
                                Some(galfus_frontend::SymbolKind::Var)
                                    | Some(galfus_frontend::SymbolKind::Const)
                            );
                            if is_global {
                                let name = res
                                    .symbol(sym)
                                    .map(|s| s.name().to_string())
                                    .unwrap_or_default();
                                let ty = self
                                    .builder
                                    .type_result
                                    .layer()
                                    .symbol_type(sym)
                                    .unwrap_or_else(|| TypeId::new(0));
                                let temp_id = self.declare_local(None, ty);
                                self.current_instructions
                                    .push(Instruction::Assign(temp_id, RValue::LoadGlobal(name)));
                                return Operand::Local(temp_id);
                            }
                        }
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

                let mut is_namespace_call = false;
                if let Some(target) = syntax.node(target_node)
                    && target.kind() == SyntaxNodeKind::PathExpression
                    && let Some(root_node) = target.first_child()
                    && let Some(root_symbol) =
                        resolution.and_then(|res| res.reference_symbol(root_node))
                    && let Some(sym_data) = resolution.and_then(|res| res.symbol(root_symbol))
                    && sym_data.kind() == SymbolKind::ImportNamespace
                {
                    is_namespace_call = true;
                }

                let func_id = if is_namespace_call {
                    FunctionId::new(target_node.raw())
                } else {
                    let symbol = resolution.and_then(|res| {
                        res.reference_symbol(target_node).or_else(|| {
                            let ident = syntax
                                .first_child_of_kind(target_node, SyntaxNodeKind::Identifier)?;
                            res.reference_symbol(ident)
                        })
                    });
                    symbol
                        .map(|sym| FunctionId::new(sym.raw()))
                        .unwrap_or_else(|| FunctionId::new(0))
                };

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
                self.lower_struct_literal(expr_id, node, statements)
            }

            SyntaxNodeKind::ArrayLiteral => self.lower_array_literal(expr_id, node, statements),

            SyntaxNodeKind::TupleExpression => self.lower_tuple_literal(expr_id, node, statements),

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
