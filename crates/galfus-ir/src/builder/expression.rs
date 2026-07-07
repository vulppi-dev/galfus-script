use super::function::FunctionBuilder;
use super::function_helpers::parse_int;
use crate::mir::*;
use galfus_core::{FunctionId, NodeId, SymbolId, TypeId};
use galfus_frontend::{PathReferenceKind, SymbolKind, SyntaxNodeKind, TypeKind};
use std::collections::HashMap;

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
                let unescaped = unescape_string(val);
                Operand::Constant(Constant::String(unescaped))
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

                let ty = self.node_type(expr_id).unwrap_or_else(|| TypeId::new(0));

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

                let ty = self.node_type(expr_id).unwrap_or_else(|| TypeId::new(0));

                let temp_id = self.declare_local(None, ty);
                self.current_instructions
                    .push(Instruction::Assign(temp_id, RValue::UnaryOp(op, operand)));
                Operand::Local(temp_id)
            }

            SyntaxNodeKind::CastExpression => {
                let type_node = node.child(0).unwrap();
                let val_node = node.child(1).unwrap();
                let operand = self.lower_expression(val_node, statements);

                let ty = self
                    .node_type(expr_id)
                    .or_else(|| self.node_type(type_node))
                    .unwrap_or_else(|| TypeId::new(0));

                let temp_id = self.declare_local(None, ty);
                self.current_instructions
                    .push(Instruction::Assign(temp_id, RValue::Cast(operand, ty)));
                Operand::Local(temp_id)
            }

            SyntaxNodeKind::CopyExpression => {
                let value_node = node.child(0).unwrap();
                let operand = self.lower_expression(value_node, statements);
                let ty = self.node_type(expr_id).unwrap_or_else(|| TypeId::new(0));
                let temp_id = self.declare_local(None, ty);
                self.current_instructions
                    .push(Instruction::Assign(temp_id, RValue::Copy(operand)));
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
                let anchored_receiver = self.anchored_call_receiver(target_node);

                let mut expected_params = None;
                if self.is_choice_variant_call_target(target_node) {
                    if let Some((_, _, payload_types)) =
                        self.get_choice_variant_payload(target_node)
                    {
                        expected_params = Some(
                            payload_types
                                .into_iter()
                                .map(|ty| (ty, false))
                                .collect::<Vec<_>>(),
                        );
                    }
                } else {
                    let target_ty = self.node_type(target_node);
                    let resolved_target_ty = target_ty.map(|t| self.builder.resolve_alias_type(t));
                    if let Some(resolved_target_ty) = resolved_target_ty
                        && let Some(TypeKind::Function(f)) = self
                            .builder
                            .type_result
                            .layer()
                            .table()
                            .kind(resolved_target_ty)
                    {
                        expected_params = Some(
                            f.parameters()
                                .iter()
                                .map(|p| (p.ty(), p.is_rest()))
                                .collect::<Vec<_>>(),
                        );
                    }
                }

                let mut args = Vec::new();
                let mut arg_types = Vec::new();
                let parameter_offset = if let Some(receiver_node) = anchored_receiver {
                    let receiver_op = self.lower_expression(receiver_node, statements);
                    let receiver_ty = self
                        .node_type(receiver_node)
                        .unwrap_or_else(|| TypeId::new(0));
                    let casted_receiver = if let Some(ref params) = expected_params
                        && let Some(&(expected_ty, _)) = params.first()
                    {
                        self.insert_cast_if_needed(receiver_op, receiver_ty, expected_ty)
                    } else {
                        receiver_op
                    };
                    args.push(casted_receiver);
                    arg_types.push(receiver_ty);
                    1
                } else {
                    0
                };

                if let Some(arg_list) = syntax.node(arg_list_node) {
                    for (i, &arg_id) in arg_list.children().iter().enumerate() {
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

                        let arg_ty = self.node_type(arg_expr).unwrap_or_else(|| TypeId::new(0));

                        let casted_op = if let Some(ref params) = expected_params {
                            let parameter_index = i + parameter_offset;
                            if let Some(&(expected_ty, is_rest)) = params.get(parameter_index) {
                                let target_expected_ty = if is_rest {
                                    let resolved_param_ty =
                                        self.builder.resolve_alias_type(expected_ty);
                                    match self
                                        .builder
                                        .type_result
                                        .layer()
                                        .table()
                                        .kind(resolved_param_ty)
                                    {
                                        Some(TypeKind::Array { element }) => *element,
                                        Some(TypeKind::FixedArray { element, .. }) => *element,
                                        _ => expected_ty,
                                    }
                                } else {
                                    expected_ty
                                };
                                self.insert_cast_if_needed(arg_op, arg_ty, target_expected_ty)
                            } else if let Some(&(expected_ty, is_rest)) = params.last() {
                                if is_rest {
                                    let resolved_param_ty =
                                        self.builder.resolve_alias_type(expected_ty);
                                    let target_expected_ty = match self
                                        .builder
                                        .type_result
                                        .layer()
                                        .table()
                                        .kind(resolved_param_ty)
                                    {
                                        Some(TypeKind::Array { element }) => *element,
                                        Some(TypeKind::FixedArray { element, .. }) => *element,
                                        _ => expected_ty,
                                    };
                                    self.insert_cast_if_needed(arg_op, arg_ty, target_expected_ty)
                                } else {
                                    arg_op
                                }
                            } else {
                                arg_op
                            }
                        } else {
                            arg_op
                        };

                        args.push(casted_op);
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

                let mut real_target = target_node;
                while let Some(node) = syntax.node(real_target)
                    && node.kind() == SyntaxNodeKind::GenericExpression
                {
                    if let Some(inner) = node.first_child() {
                        real_target = inner;
                    } else {
                        break;
                    }
                }

                let mut is_namespace_call = false;
                if let Some(target) = syntax.node(real_target)
                    && target.kind() == SyntaxNodeKind::PathExpression
                    && let Some(root_node) = target.first_child()
                    && let Some(root_symbol) =
                        resolution.and_then(|res| res.reference_symbol(root_node))
                    && let Some(sym_data) = resolution.and_then(|res| res.symbol(root_symbol))
                    && sym_data.kind() == SymbolKind::ImportNamespace
                {
                    is_namespace_call = true;
                }

                let target_symbol = self.call_target_symbol(target_node).or_else(|| {
                    anchored_receiver
                        .and_then(|receiver| self.anchored_function_symbol(receiver, target_node))
                });

                // Detect constraint method call: receiver is of a constraint type.
                // Example: `item::stringify()` where `item: Stringable`.
                // The receiver is child(0) of the PathExpression; its type resolves to
                // a Named type whose symbol has SymbolKind::Constraint.
                let is_constraint_method = syntax
                    .node(real_target)
                    .and_then(|target| {
                        if target.kind() != SyntaxNodeKind::PathExpression {
                            return None;
                        }
                        let receiver_node = target.child(0)?;
                        // receiver must be a name expression (local variable), not a module namespace
                        let receiver_kind = syntax.node(receiver_node)?.kind();
                        if !matches!(
                            receiver_kind,
                            SyntaxNodeKind::NameExpression | SyntaxNodeKind::Identifier
                        ) {
                            return None;
                        }
                        // Look up the type of the receiver
                        let receiver_ty =
                            self.builder.type_result.layer().node_type(receiver_node)?;
                        let receiver_ty = self.builder.resolve_alias_type(receiver_ty);
                        // Check if the type is Named and its symbol is a Constraint
                        if let Some(TypeKind::Named { symbol }) =
                            self.builder.type_result.layer().table().kind(receiver_ty)
                        {
                            let sym_data = self.builder.graph.resolution()?.symbol(*symbol)?;
                            if sym_data.kind() == SymbolKind::Constraint {
                                return Some(());
                            }
                        }
                        None
                    })
                    .is_some();

                if is_constraint_method {
                    // Extract method name from the PathExpression member node (child 1).
                    let method_name = syntax
                        .node(real_target)
                        .and_then(|n| n.child(1))
                        .and_then(|member_node| {
                            syntax.node(member_node).map(|mn| {
                                let span = mn.span();
                                if (span.start() as usize) < self.builder.source_text.len()
                                    && (span.end() as usize) <= self.builder.source_text.len()
                                {
                                    self.builder.source_text
                                        [span.start() as usize..span.end() as usize]
                                        .to_string()
                                } else {
                                    String::new()
                                }
                            })
                        })
                        .unwrap_or_default();

                    // For `item::stringify()`, the receiver `item` is child(0) of the
                    // PathExpression. `anchored_receiver` is None (item is NameExpression),
                    // so args is empty. We must lower the receiver node directly.
                    let obj = if !args.is_empty() {
                        // Receiver was already lowered as args[0] (anchored_receiver case).
                        args[0].clone()
                    } else if let Some(receiver_node) =
                        syntax.node(real_target).and_then(|n| n.child(0))
                    {
                        self.lower_expression(receiver_node, statements)
                    } else {
                        Operand::Constant(Constant::Null)
                    };
                    // Extra args (beyond the receiver) are args[1..] or all of args if
                    // no receiver was pre-lowered.
                    let extra_args = if !args.is_empty() {
                        args[1..].to_vec()
                    } else {
                        vec![]
                    };

                    let ty = self.node_type(expr_id).unwrap_or_else(|| TypeId::new(0));
                    let temp_id = self.declare_local(None, ty);

                    let instructions = std::mem::take(&mut self.current_instructions);
                    statements.push(MirBody::BasicBlock(BasicBlock {
                        id: self.builder.next_block(),
                        instructions,
                        terminator: Terminator::ConstraintCall {
                            method_name,
                            obj,
                            args: extra_args,
                            destination: temp_id,
                        },
                    }));

                    return Operand::Local(temp_id);
                }

                let mut func_id = if is_namespace_call {
                    path_call_function_id(real_target)
                } else if anchored_receiver.is_some() {
                    target_symbol
                        .map(|sym| FunctionId::new(sym.raw()))
                        .unwrap_or_else(|| path_call_function_id(real_target))
                } else {
                    target_symbol
                        .map(|sym| FunctionId::new(sym.raw()))
                        .unwrap_or_else(|| FunctionId::new(0))
                };

                if !self.is_std_buffer_create_call_target(target_node)
                    && let Some(symbol) = target_symbol
                    && let Some(specialized) =
                        self.specialize_generic_call(symbol, target_node, &arg_types)
                {
                    func_id = specialized;
                }

                let ty = self.node_type(expr_id).unwrap_or_else(|| TypeId::new(0));

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

                let ty = self.node_type(expr_id).unwrap_or_else(|| TypeId::new(0));

                let temp_id = self.declare_local(None, ty);
                let obj_ty = self.node_type(obj_node).unwrap_or_else(|| TypeId::new(0));
                let resolved_obj_ty = self.builder.resolve_alias_type(obj_ty);
                let is_array_length = member_name == "length"
                    && matches!(
                        self.builder
                            .type_result
                            .layer()
                            .table()
                            .kind(resolved_obj_ty),
                        Some(TypeKind::Array { .. }) | Some(TypeKind::FixedArray { .. })
                    );

                let rval = if is_array_length {
                    RValue::Len(obj_operand)
                } else {
                    RValue::MemberAccess(obj_operand, member_name)
                };

                self.current_instructions
                    .push(Instruction::Assign(temp_id, rval));
                Operand::Local(temp_id)
            }

            SyntaxNodeKind::IndexExpression => {
                let target_node = node.child(0).unwrap();
                let index_node = node.child(1).unwrap();

                let target_operand = self.lower_expression(target_node, statements);
                let index_operand = self.lower_expression(index_node, statements);

                let ty = self.node_type(expr_id).unwrap_or_else(|| TypeId::new(0));

                let target_ty = self
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
                    .node_type(subject_node)
                    .unwrap_or_else(|| TypeId::new(0));

                let match_type = self.node_type(expr_id).unwrap_or_else(|| TypeId::new(0));

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

            SyntaxNodeKind::NewArrayExpression => {
                self.lower_new_array_expression(expr_id, node, statements)
            }

            _ => Operand::Constant(Constant::Null),
        }
    }

    fn call_target_symbol(&self, target: NodeId) -> Option<SymbolId> {
        let syntax = self.builder.graph.syntax();
        let resolution = self.builder.graph.resolution()?;
        let node = syntax.node(target)?;

        match node.kind() {
            SyntaxNodeKind::NameExpression => resolution.reference_symbol(target).or_else(|| {
                let ident = syntax.first_child_of_kind(target, SyntaxNodeKind::Identifier)?;
                resolution.reference_symbol(ident)
            }),
            SyntaxNodeKind::PathExpression => resolution
                .path_reference_symbol(target)
                .or_else(|| resolution.reference_symbol(target)),
            SyntaxNodeKind::GenericExpression => syntax
                .child(target, 0)
                .and_then(|inner| self.call_target_symbol(inner)),
            _ => None,
        }
    }

    fn is_std_buffer_create_call_target(&self, target: NodeId) -> bool {
        let syntax = self.builder.graph.syntax();
        let Some(resolution) = self.builder.graph.resolution() else {
            return false;
        };

        let mut current = target;
        while let Some(node) = syntax.node(current)
            && node.kind() == SyntaxNodeKind::GenericExpression
        {
            if let Some(inner) = node.first_child() {
                current = inner;
            } else {
                break;
            }
        }

        let Some(node) = syntax.node(current) else {
            return false;
        };

        if node.kind() != SyntaxNodeKind::PathExpression {
            return false;
        }

        let Some(root_node) = node.child(0) else {
            return false;
        };
        let Some(member_node) = node.child(1) else {
            return false;
        };

        if self.builder.node_text(member_node) != "create" {
            return false;
        }

        let root_symbol = resolution.reference_symbol(root_node).or_else(|| {
            let identifier = syntax.first_child_of_kind(root_node, SyntaxNodeKind::Identifier)?;
            resolution.reference_symbol(identifier)
        });

        root_symbol
            .and_then(|symbol| resolution.import_for_symbol(symbol))
            .and_then(|import_id| resolution.import(import_id))
            .is_some_and(|import| import.source() == "std/buffer")
    }

    fn anchored_call_receiver(&self, target: NodeId) -> Option<NodeId> {
        if self.is_choice_variant_call_target(target) {
            return None;
        }

        let syntax = self.builder.graph.syntax();
        let node = syntax.node(target)?;
        if node.kind() != SyntaxNodeKind::PathExpression {
            return None;
        }

        let receiver = node.child(0)?;
        let receiver_kind = syntax.node(receiver)?.kind();
        if matches!(
            receiver_kind,
            SyntaxNodeKind::NameExpression
                | SyntaxNodeKind::Identifier
                | SyntaxNodeKind::Path
                | SyntaxNodeKind::GenericExpression
        ) {
            None
        } else {
            Some(receiver)
        }
    }

    fn anchored_function_symbol(&self, receiver: NodeId, target: NodeId) -> Option<SymbolId> {
        let syntax = self.builder.graph.syntax();
        let resolution = self.builder.graph.resolution()?;
        let member = syntax.child(target, 1)?;
        let member_name = self.builder.node_text(member);

        let receiver_ty = self.node_type(receiver)?;
        let receiver_ty = self.builder.resolve_alias_type(receiver_ty);
        let TypeKind::Named { symbol } =
            self.builder.type_result.layer().table().kind(receiver_ty)?
        else {
            return None;
        };

        let receiver_symbol = resolution.symbol(*symbol)?;
        if receiver_symbol.kind() != SymbolKind::Struct {
            return None;
        }

        let function_name = format!("{}::{}", receiver_symbol.name(), member_name);
        resolution
            .symbols()
            .iter()
            .find(|symbol| {
                symbol.kind() == SymbolKind::Function && symbol.name() == function_name.as_str()
            })
            .map(|symbol| symbol.id())
    }

    fn specialize_generic_call(
        &mut self,
        symbol: SymbolId,
        target_node: NodeId,
        arg_types: &[TypeId],
    ) -> Option<FunctionId> {
        let original_id = FunctionId::new(symbol.raw());
        let function_item = self.builder.function_item_for_symbol(symbol);

        if let Some(function_item) = function_item {
            let generic_params = self
                .builder
                .generic_parameters_for_function_item(function_item);

            if generic_params.is_empty() {
                return None;
            }

            let concrete_types =
                self.concrete_generic_arguments(target_node, &generic_params, arg_types)?;
            if concrete_types.len() != generic_params.len() {
                return None;
            }

            let key = (original_id, concrete_types.clone());

            if let Some(func_id) = self.builder.specialisations.get(&key).copied() {
                return Some(func_id);
            }

            if self.builder.active_specialisations.contains(&key) {
                return Some(original_id);
            }

            let specialized_id = self.builder.next_specialized_function_id();
            self.builder
                .specialisations
                .insert(key.clone(), specialized_id);
            self.builder.active_specialisations.insert(key.clone());

            let substitutions = generic_params
                .into_iter()
                .zip(concrete_types)
                .collect::<HashMap<_, _>>();

            let caller_next_local = self.builder.next_local_id;
            if let Some(mut function) = self.builder.build_function_with_substitutions(
                function_item,
                Some(specialized_id),
                substitutions,
            ) {
                function.name = format!("{}#{}", function.name, specialized_id.raw());
                self.builder.specialized_functions.push(function);
            }
            self.builder.next_local_id = caller_next_local;

            self.builder.active_specialisations.remove(&key);

            Some(specialized_id)
        } else if let Some(ctx_ptr) = self.builder.workspace_ctx {
            let ctx = unsafe { &mut *ctx_ptr };
            if let Some((target_mod_idx, target_symbol)) = ctx.resolve_import(target_node) {
                if let Some(generic_params) = ctx.get_generic_params(target_mod_idx, target_symbol)
                {
                    if generic_params.is_empty() {
                        return None;
                    }

                    let concrete_types =
                        self.concrete_generic_arguments(target_node, &generic_params, arg_types)?;
                    if concrete_types.len() != generic_params.len() {
                        return None;
                    }

                    let substitutions = generic_params
                        .into_iter()
                        .zip(concrete_types.clone())
                        .collect::<HashMap<_, _>>();

                    let specialized_id = ctx.specialize_function(
                        target_node,
                        target_mod_idx,
                        target_symbol,
                        concrete_types,
                        substitutions,
                    );
                    Some(specialized_id)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn concrete_generic_arguments(
        &self,
        target_node: NodeId,
        generic_params: &[SymbolId],
        arg_types: &[TypeId],
    ) -> Option<Vec<TypeId>> {
        let syntax = self.builder.graph.syntax();

        if syntax
            .node(target_node)
            .is_some_and(|node| node.kind() == SyntaxNodeKind::GenericExpression)
            && let Some(argument_list) = syntax.child(target_node, 1)
            && let Some(argument_node) = syntax.node(argument_list)
        {
            let explicit = argument_node
                .children()
                .iter()
                .filter_map(|argument| {
                    self.node_type(*argument).or_else(|| {
                        self.first_type_child(*argument)
                            .and_then(|type_node| self.node_type(type_node))
                    })
                })
                .collect::<Vec<_>>();

            if !explicit.is_empty() {
                return Some(explicit);
            }
        }

        self.infer_generic_arguments_from_call(target_node, generic_params, arg_types)
    }

    fn infer_generic_arguments_from_call(
        &self,
        target_node: NodeId,
        generic_params: &[SymbolId],
        arg_types: &[TypeId],
    ) -> Option<Vec<TypeId>> {
        let target_ty = self.node_type(target_node)?;
        let target_ty = self.builder.resolve_alias_type(target_ty);
        let TypeKind::Function(function) =
            self.builder.type_result.layer().table().kind(target_ty)?
        else {
            return None;
        };

        let mut substitutions = HashMap::new();

        for (parameter, &arg_ty) in function.parameters().iter().zip(arg_types.iter()) {
            self.infer_generic_argument_from_types(
                generic_params,
                parameter.ty(),
                arg_ty,
                &mut substitutions,
            );
        }

        generic_params
            .iter()
            .map(|param| substitutions.get(param).copied())
            .collect()
    }

    fn infer_generic_argument_from_types(
        &self,
        generic_params: &[SymbolId],
        parameter_ty: TypeId,
        argument_ty: TypeId,
        substitutions: &mut HashMap<SymbolId, TypeId>,
    ) {
        let parameter_ty = self.builder.resolve_alias_type(parameter_ty);
        let argument_ty = self.builder.resolve_alias_type(argument_ty);

        match self.builder.type_result.layer().table().kind(parameter_ty) {
            Some(TypeKind::GenericParameter { symbol }) if generic_params.contains(symbol) => {
                substitutions.entry(*symbol).or_insert(argument_ty);
            }
            Some(TypeKind::Array { element }) | Some(TypeKind::FixedArray { element, .. }) => {
                if let Some(TypeKind::Array {
                    element: arg_element,
                })
                | Some(TypeKind::FixedArray {
                    element: arg_element,
                    ..
                }) = self.builder.type_result.layer().table().kind(argument_ty)
                {
                    self.infer_generic_argument_from_types(
                        generic_params,
                        *element,
                        *arg_element,
                        substitutions,
                    );
                }
            }
            Some(TypeKind::Tuple { elements }) => {
                if let Some(TypeKind::Tuple {
                    elements: arg_elements,
                }) = self.builder.type_result.layer().table().kind(argument_ty)
                {
                    for (element, arg_element) in elements.iter().zip(arg_elements.iter()) {
                        self.infer_generic_argument_from_types(
                            generic_params,
                            *element,
                            *arg_element,
                            substitutions,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

const PATH_CALL_TARGET_TAG: u32 = 0x8000_0000;

fn path_call_function_id(node: NodeId) -> FunctionId {
    FunctionId::new(PATH_CALL_TARGET_TAG | node.raw())
}

pub(crate) fn unescape_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('"') => result.push('"'),
                Some('\'') => result.push('\''),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}
