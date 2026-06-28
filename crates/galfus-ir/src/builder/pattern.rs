use super::function::FunctionBuilder;
use crate::mir::*;
use galfus_core::{NodeId, SymbolId, TypeId};
use galfus_frontend::{SymbolKind, SyntaxNodeKind, TypeKind};

impl<'b, 'a> FunctionBuilder<'b, 'a> {
    pub(super) fn variant_pattern_symbols(&self, pattern: NodeId) -> Option<(SymbolId, SymbolId)> {
        let resolution = self.builder.graph.resolution()?;
        let owner_symbol = resolution.reference_symbol(pattern)?;
        let variant_symbol = resolution.path_reference_symbol(pattern)?;
        Some((owner_symbol, variant_symbol))
    }

    pub(super) fn lower_match_arms(
        &mut self,
        arms: &[NodeId],
        index: usize,
        subject: &Operand,
        result_local: LocalId,
    ) -> MirBody {
        let syntax = self.builder.graph.syntax();
        if index >= arms.len() {
            let insts = vec![Instruction::Assign(
                result_local,
                RValue::Use(Operand::Constant(Constant::Null)),
            )];
            return MirBody::BasicBlock(BasicBlock {
                id: self.builder.next_block(),
                instructions: insts,
                terminator: Terminator::None,
            });
        }

        let arm_node = arms[index];
        let pattern_node = syntax.child(arm_node, 0).unwrap();
        let body_node = syntax.child(arm_node, 1).unwrap();

        let mut check_statements = Vec::new();
        let mut bindings = Vec::new();
        let cond_op =
            self.lower_pattern_check(pattern_node, subject, &mut check_statements, &mut bindings);

        let mut then_statements = Vec::new();
        then_statements.extend(bindings);

        let body_op = self.lower_expression(body_node, &mut then_statements);
        self.current_instructions
            .push(Instruction::Assign(result_local, RValue::Use(body_op)));
        self.flush_current_instructions(&mut then_statements);

        let then_branch = Box::new(if then_statements.len() == 1 {
            then_statements.pop().unwrap()
        } else {
            MirBody::Block {
                locals: Vec::new(),
                statements: then_statements,
            }
        });

        let else_branch = Some(Box::new(self.lower_match_arms(
            arms,
            index + 1,
            subject,
            result_local,
        )));

        let if_body = MirBody::If {
            cond: cond_op,
            then_branch,
            else_branch,
        };

        if check_statements.is_empty() {
            if_body
        } else {
            let mut all_statements = check_statements;
            all_statements.push(if_body);
            MirBody::Block {
                locals: Vec::new(),
                statements: all_statements,
            }
        }
    }

    pub(super) fn lower_pattern_check(
        &mut self,
        pattern_node_id: NodeId,
        subject: &Operand,
        statements: &mut Vec<MirBody>,
        bindings: &mut Vec<MirBody>,
    ) -> Operand {
        let syntax = self.builder.graph.syntax();
        let pattern_node = syntax.node(pattern_node_id).unwrap();
        let resolution = self.builder.graph.resolution();

        match pattern_node.kind() {
            SyntaxNodeKind::LiteralPattern => {
                let literal_expr = syntax.child(pattern_node_id, 0).unwrap();
                let literal_op = self.lower_expression(literal_expr, statements);
                let bool_ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(pattern_node_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let cond_temp = self.declare_local(None, bool_ty);
                self.current_instructions.push(Instruction::Assign(
                    cond_temp,
                    RValue::BinaryOp(MirBinaryOp::Equal, subject.clone(), literal_op),
                ));
                Operand::Local(cond_temp)
            }

            SyntaxNodeKind::WildcardPattern => Operand::Constant(Constant::Bool(true)),

            SyntaxNodeKind::BindingPattern => {
                if let Some(res) = resolution {
                    let ident = syntax
                        .first_child_of_kind(pattern_node_id, SyntaxNodeKind::Identifier)
                        .unwrap_or(pattern_node_id);
                    if let Some(symbol) = res.declaration_symbol(ident) {
                        let ty = self
                            .builder
                            .type_result
                            .layer()
                            .symbol_type(symbol)
                            .unwrap_or_else(|| TypeId::new(0));
                        let local_id = self.declare_local(Some(symbol), ty);
                        self.symbol_to_local.insert(symbol, local_id);

                        let bind_insts =
                            vec![Instruction::Assign(local_id, RValue::Use(subject.clone()))];
                        bindings.push(MirBody::BasicBlock(BasicBlock {
                            id: self.builder.next_block(),
                            instructions: bind_insts,
                            terminator: Terminator::None,
                        }));
                    }
                }
                Operand::Constant(Constant::Bool(true))
            }

            SyntaxNodeKind::VariantPattern => {
                let symbols = self.variant_pattern_symbols(pattern_node_id);
                let variant_data =
                    symbols.and_then(|(_, vs)| resolution.and_then(|res| res.symbol(vs)));
                if let (Some((owner_symbol, variant_symbol)), Some(variant_data)) =
                    (symbols, variant_data)
                {
                    match variant_data.kind() {
                        SymbolKind::EnumVariant => {
                            let val = self.get_enum_variant_value(variant_symbol);
                            let bool_ty = self
                                .builder
                                .type_result
                                .layer()
                                .node_type(pattern_node_id)
                                .unwrap_or_else(|| TypeId::new(0));
                            let cond_temp = self.declare_local(None, bool_ty);
                            self.current_instructions.push(Instruction::Assign(
                                cond_temp,
                                RValue::BinaryOp(
                                    MirBinaryOp::Equal,
                                    subject.clone(),
                                    Operand::Constant(Constant::Int(val)),
                                ),
                            ));
                            return Operand::Local(cond_temp);
                        }
                        SymbolKind::ChoiceVariant => {
                            let variant_name = variant_data.name().to_string();
                            let mut variant_ty = TypeId::new(0);
                            let table = self.builder.type_result.layer().table();
                            for id in 0..table.len() {
                                let ty_id = TypeId::new(id as u32);
                                if matches!(table.kind(ty_id), Some(TypeKind::Named { symbol }) if *symbol == variant_symbol)
                                {
                                    variant_ty = ty_id;
                                    break;
                                }
                            }

                            let bool_ty = self
                                .builder
                                .type_result
                                .layer()
                                .node_type(pattern_node_id)
                                .unwrap_or_else(|| TypeId::new(0));

                            let cond_temp = self.declare_local(None, bool_ty);
                            self.current_instructions.push(Instruction::Assign(
                                cond_temp,
                                RValue::Instanceof(subject.clone(), variant_ty),
                            ));

                            if let Some(payload_node_id) = syntax.first_child_of_kind(
                                pattern_node_id,
                                SyntaxNodeKind::VariantPatternPayload,
                            ) {
                                let payload_node = syntax.node(payload_node_id).unwrap();
                                let payload_patterns = payload_node.children();

                                let payload_types =
                                    self.choice_variant_payload_types(owner_symbol, variant_symbol);

                                if !payload_patterns.is_empty() {
                                    let payload_ty = if payload_patterns.len() > 1 {
                                        self.find_tuple_type(&payload_types)
                                    } else {
                                        payload_types[0]
                                    };

                                    let payload_temp = self.declare_local(None, payload_ty);
                                    let extract_insts = vec![Instruction::Assign(
                                        payload_temp,
                                        RValue::MemberAccess(subject.clone(), variant_name),
                                    )];

                                    let payload_op = Operand::Local(payload_temp);
                                    if payload_patterns.len() == 1 {
                                        let mut nested_bindings = Vec::new();
                                        let _ = self.lower_pattern_check(
                                            payload_patterns[0],
                                            &payload_op,
                                            bindings,
                                            &mut nested_bindings,
                                        );
                                        bindings.push(MirBody::BasicBlock(BasicBlock {
                                            id: self.builder.next_block(),
                                            instructions: extract_insts,
                                            terminator: Terminator::None,
                                        }));
                                        bindings.extend(nested_bindings);
                                    } else {
                                        bindings.push(MirBody::BasicBlock(BasicBlock {
                                            id: self.builder.next_block(),
                                            instructions: extract_insts,
                                            terminator: Terminator::None,
                                        }));
                                        for (i, &child_pattern) in
                                            payload_patterns.iter().enumerate()
                                        {
                                            let element_ty = payload_types[i];
                                            let element_temp = self.declare_local(None, element_ty);
                                            let elem_insts = vec![Instruction::Assign(
                                                element_temp,
                                                RValue::MemberAccess(
                                                    payload_op.clone(),
                                                    i.to_string(),
                                                ),
                                            )];
                                            bindings.push(MirBody::BasicBlock(BasicBlock {
                                                id: self.builder.next_block(),
                                                instructions: elem_insts,
                                                terminator: Terminator::None,
                                            }));

                                            let mut nested_bindings = Vec::new();
                                            let _ = self.lower_pattern_check(
                                                child_pattern,
                                                &Operand::Local(element_temp),
                                                bindings,
                                                &mut nested_bindings,
                                            );
                                            bindings.extend(nested_bindings);
                                        }
                                    }
                                }
                            }

                            return Operand::Local(cond_temp);
                        }
                        _ => {}
                    }
                }
                Operand::Constant(Constant::Bool(false))
            }

            SyntaxNodeKind::TypePattern => {
                let type_node = self.first_type_child(pattern_node_id).unwrap();
                let pattern_type = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(type_node)
                    .unwrap_or_else(|| TypeId::new(0));

                let bool_ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(pattern_node_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let cond_temp = self.declare_local(None, bool_ty);
                self.current_instructions.push(Instruction::Assign(
                    cond_temp,
                    RValue::Instanceof(subject.clone(), pattern_type),
                ));

                if let Some(binding_node_id) = syntax
                    .first_child_of_kind(pattern_node_id, SyntaxNodeKind::TypePatternBinding)
                    .filter(|_| resolution.is_some())
                {
                    let symbols = self.declaration_symbols_in_node(
                        binding_node_id,
                        &[SymbolKind::TypePatternBinding],
                    );
                    for symbol in symbols {
                        let local_id = self.declare_local(Some(symbol), pattern_type);
                        self.symbol_to_local.insert(symbol, local_id);

                        let bind_insts =
                            vec![Instruction::Assign(local_id, RValue::Use(subject.clone()))];
                        bindings.push(MirBody::BasicBlock(BasicBlock {
                            id: self.builder.next_block(),
                            instructions: bind_insts,
                            terminator: Terminator::None,
                        }));
                    }
                }

                Operand::Local(cond_temp)
            }

            _ => Operand::Constant(Constant::Null),
        }
    }
}
