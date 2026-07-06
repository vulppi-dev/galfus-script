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

    fn get_imported_choice_variant(&self, pattern: NodeId) -> Option<(String, Vec<TypeId>)> {
        let variant_ty = self.builder.type_result.layer().node_type(pattern)?;
        let table = self.builder.type_result.layer().table();
        let (_root, segments) = match table.kind(variant_ty) {
            Some(TypeKind::Path { root, segments }) => (*root, segments),
            _ => return None,
        };
        if segments.len() != 2 {
            return None;
        }
        let choice_name = &segments[0];
        let variant_name = &segments[1];

        let choice = self
            .builder
            .type_result
            .imported_path_choices
            .values()
            .find(|c| c.name == *choice_name)?;
        let variant = choice.variants.iter().find(|v| v.name == *variant_name)?;

        Some((variant.name.clone(), variant.payload_types.clone()))
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
        self.flush_current_instructions(&mut check_statements);

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
                            let mut variant_ty = self
                                .builder
                                .type_result
                                .layer()
                                .node_type(pattern_node_id)
                                .unwrap_or_else(|| TypeId::new(0));

                            if variant_ty == TypeId::new(0) {
                                let table = self.builder.type_result.layer().table();
                                for id in 0..table.len() {
                                    let ty_id = TypeId::new(id as u32);
                                    if matches!(table.kind(ty_id), Some(TypeKind::Named { symbol }) if *symbol == variant_symbol)
                                    {
                                        variant_ty = ty_id;
                                        break;
                                    }
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

                                let payload_types = if let Some((_, imported_payload_types)) =
                                    self.get_imported_choice_variant(pattern_node_id)
                                {
                                    imported_payload_types
                                } else {
                                    self.choice_variant_payload_types(owner_symbol, variant_symbol)
                                };

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
                } else if let Some((variant_name, payload_types)) =
                    self.get_imported_choice_variant(pattern_node_id)
                {
                    let variant_ty = self
                        .builder
                        .type_result
                        .layer()
                        .node_type(pattern_node_id)
                        .unwrap();

                    let bool_ty = self
                        .builder
                        .type_result
                        .layer()
                        .table()
                        .primitive(galfus_frontend::PrimitiveType::Bool);

                    let cond_temp = self.declare_local(None, bool_ty);
                    self.current_instructions.push(Instruction::Assign(
                        cond_temp,
                        RValue::Instanceof(subject.clone(), variant_ty),
                    ));

                    if let Some(payload_node_id) = syntax
                        .first_child_of_kind(pattern_node_id, SyntaxNodeKind::VariantPatternPayload)
                    {
                        let payload_node = syntax.node(payload_node_id).unwrap();
                        let payload_patterns = payload_node.children();

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
                                for (i, &child_pattern) in payload_patterns.iter().enumerate() {
                                    let element_ty = payload_types[i];
                                    let element_temp = self.declare_local(None, element_ty);
                                    let elem_insts = vec![Instruction::Assign(
                                        element_temp,
                                        RValue::MemberAccess(payload_op.clone(), i.to_string()),
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

            SyntaxNodeKind::StructPattern => {
                let pattern_type = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(pattern_node_id)
                    .unwrap_or_else(|| TypeId::new(0));

                let bool_ty = self
                    .builder
                    .type_result
                    .layer()
                    .table()
                    .primitive(galfus_frontend::PrimitiveType::Bool);

                let cond_temp = self.declare_local(None, bool_ty);
                self.current_instructions.push(Instruction::Assign(
                    cond_temp,
                    RValue::Instanceof(subject.clone(), pattern_type),
                ));

                let mut and_conditions = Vec::new();
                and_conditions.push(Operand::Local(cond_temp));

                for &field in &pattern_node.children()[1..] {
                    let field_node = syntax.node(field).unwrap();
                    let field_ident = syntax
                        .first_child_of_kind(field, SyntaxNodeKind::Identifier)
                        .unwrap();
                    let field_name = self.builder.node_text(field_ident).to_string();

                    let field_ty = self
                        .builder
                        .type_result
                        .layer()
                        .node_type(field)
                        .unwrap_or_else(|| TypeId::new(0));

                    let field_temp = self.declare_local(None, field_ty);
                    let extract_inst = Instruction::Assign(
                        field_temp,
                        RValue::MemberAccess(subject.clone(), field_name),
                    );

                    bindings.push(MirBody::BasicBlock(BasicBlock {
                        id: self.builder.next_block(),
                        instructions: vec![extract_inst],
                        terminator: Terminator::None,
                    }));

                    let field_op = Operand::Local(field_temp);

                    if field_node.children().len() > 1 {
                        // Aliased: e.g. `x: pattern`
                        let inner_pattern = field_node.child(1).unwrap();
                        let mut inner_bindings = Vec::new();
                        let inner_cond = self.lower_pattern_check(
                            inner_pattern,
                            &field_op,
                            statements,
                            &mut inner_bindings,
                        );
                        and_conditions.push(inner_cond);
                        bindings.extend(inner_bindings);
                    } else {
                        // Shorthand: binds the variable to local
                        if let Some(res) = resolution {
                            let ident = syntax
                                .first_child_of_kind(field, SyntaxNodeKind::Identifier)
                                .unwrap();
                            if let Some(symbol) = res.declaration_symbol(ident) {
                                let local_id = self.declare_local(Some(symbol), field_ty);
                                self.symbol_to_local.insert(symbol, local_id);
                                let bind_inst =
                                    Instruction::Assign(local_id, RValue::Use(field_op));
                                bindings.push(MirBody::BasicBlock(BasicBlock {
                                    id: self.builder.next_block(),
                                    instructions: vec![bind_inst],
                                    terminator: Terminator::None,
                                }));
                            }
                        }
                    }
                }

                let mut final_cond = and_conditions[0].clone();
                for next_cond in &and_conditions[1..] {
                    let temp = self.declare_local(None, bool_ty);
                    self.current_instructions.push(Instruction::Assign(
                        temp,
                        RValue::BinaryOp(MirBinaryOp::LogicalAnd, final_cond, next_cond.clone()),
                    ));
                    final_cond = Operand::Local(temp);
                }

                final_cond
            }

            _ => Operand::Constant(Constant::Null),
        }
    }
}
