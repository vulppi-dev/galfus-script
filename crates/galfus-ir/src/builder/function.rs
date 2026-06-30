use crate::mir::*;
use galfus_core::{NodeId, SymbolId, TypeId};
use galfus_frontend::{PrimitiveType, SyntaxNodeKind, TypeKind};

pub struct FunctionBuilder<'b, 'a> {
    pub(super) builder: &'b mut super::MirBuilder<'a>,
    pub(super) locals: Vec<LocalDecl>,
    pub(super) symbol_to_local: std::collections::HashMap<SymbolId, LocalId>,
    pub(super) current_instructions: Vec<Instruction>,
    pub(super) scopes: Vec<Vec<LocalId>>,
    pub(super) return_type: TypeId,
}

impl<'b, 'a> FunctionBuilder<'b, 'a> {
    pub(super) fn declare_local(&mut self, symbol: Option<SymbolId>, ty: TypeId) -> LocalId {
        let local_id = self.builder.next_local();
        self.locals.push(LocalDecl { id: local_id, ty });
        if let Some(sym) = symbol {
            self.symbol_to_local.insert(sym, local_id);
        }
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.push(local_id);
        }
        local_id
    }

    pub(super) fn collect_declaration_symbols(&self, node_id: NodeId) -> Vec<SymbolId> {
        let mut symbols = Vec::new();
        self.collect_symbols_recursive(node_id, &mut symbols);
        symbols
    }

    pub(super) fn collect_symbols_recursive(&self, node_id: NodeId, symbols: &mut Vec<SymbolId>) {
        if let Some(sym) = self
            .builder
            .graph
            .resolution()
            .and_then(|res| res.declaration_symbol(node_id))
        {
            symbols.push(sym);
        }
        if let Some(node) = self.builder.graph.syntax().node(node_id) {
            for &child in node.children() {
                self.collect_symbols_recursive(child, symbols);
            }
        }
    }

    fn is_terminated(statements: &[MirBody]) -> bool {
        if let Some(MirBody::BasicBlock(bb)) = statements.last() {
            matches!(bb.terminator, Terminator::Return(_))
        } else {
            false
        }
    }

    pub(super) fn lower_block(&mut self, block_node_id: NodeId) -> MirBody {
        let syntax = self.builder.graph.syntax();
        let Some(block_node) = syntax.node(block_node_id) else {
            return MirBody::BasicBlock(BasicBlock {
                id: self.builder.next_block(),
                instructions: Vec::new(),
                terminator: Terminator::Return(None),
            });
        };

        self.scopes.push(Vec::new());
        let mut statements = Vec::new();

        for &stmt_id in block_node.children() {
            self.lower_statement(stmt_id, &mut statements);
        }

        if let Some(scope_locals) = self.scopes.pop()
            && !Self::is_terminated(&statements)
        {
            for local_id in scope_locals {
                if let Some(decl) = self.locals.iter().find(|l| l.id == local_id)
                    && self.builder.is_owned_type(decl.ty)
                {
                    self.current_instructions.push(Instruction::Drop(local_id));
                }
            }
        }

        self.flush_current_instructions(&mut statements);

        if statements.len() == 1 {
            statements.pop().unwrap()
        } else {
            MirBody::Block {
                locals: Vec::new(),
                statements,
            }
        }
    }

    pub(super) fn flush_current_instructions(&mut self, statements: &mut Vec<MirBody>) {
        if !self.current_instructions.is_empty() {
            let instructions = std::mem::take(&mut self.current_instructions);
            statements.push(MirBody::BasicBlock(BasicBlock {
                id: self.builder.next_block(),
                instructions,
                terminator: Terminator::None,
            }));
        }
    }

    fn index_assignment_value_type(&self, target: NodeId) -> Option<TypeId> {
        let syntax = self.builder.graph.syntax();
        let target_node = syntax.node(target)?;

        if target_node.kind() != SyntaxNodeKind::IndexExpression {
            return None;
        }

        let array_node = target_node.child(0)?;
        let array_type = self.builder.type_result.layer().node_type(array_node)?;
        let resolved_array_type = self.builder.resolve_alias_type(array_type);

        match self
            .builder
            .type_result
            .layer()
            .table()
            .kind(resolved_array_type)
        {
            Some(TypeKind::Array { element }) => Some(*element),
            Some(TypeKind::FixedArray { element, .. }) => Some(*element),
            _ => self.builder.type_result.layer().node_type(target),
        }
    }

    fn lower_index_assignment(
        &mut self,
        target: NodeId,
        value: NodeId,
        statements: &mut Vec<MirBody>,
    ) -> bool {
        let syntax = self.builder.graph.syntax();

        let Some(target_node) = syntax.node(target) else {
            return false;
        };

        if target_node.kind() != SyntaxNodeKind::IndexExpression {
            return false;
        }

        let Some(array_node) = target_node.child(0) else {
            return true;
        };

        let Some(index_node) = target_node.child(1) else {
            return true;
        };

        let array_operand = self.lower_expression(array_node, statements);
        let index_operand = self.lower_expression(index_node, statements);
        let value_operand = self.lower_expression(value, statements);

        let value_type = self
            .builder
            .type_result
            .layer()
            .node_type(value)
            .unwrap_or_else(|| TypeId::new(0));

        let expected_value_type = self
            .index_assignment_value_type(target)
            .unwrap_or_else(|| TypeId::new(0));

        let casted_value =
            self.insert_cast_if_needed(value_operand, value_type, expected_value_type);

        self.current_instructions.push(Instruction::StoreIndex {
            arr: array_operand,
            idx: index_operand,
            val: casted_value,
        });

        true
    }

    pub(super) fn lower_statement(&mut self, stmt_id: NodeId, statements: &mut Vec<MirBody>) {
        let syntax = self.builder.graph.syntax();
        let Some(node) = syntax.node(stmt_id) else {
            return;
        };
        let resolution = self.builder.graph.resolution();

        match node.kind() {
            SyntaxNodeKind::VarStatement | SyntaxNodeKind::ConstStatement => {
                if let Some(binding) =
                    syntax.first_child_of_kind(stmt_id, SyntaxNodeKind::BindingPattern)
                {
                    let initializer = syntax
                        .first_child_of_kind(stmt_id, SyntaxNodeKind::Initializer)
                        .and_then(|init| syntax.first_child(init));

                    let operand = if let Some(init_expr) = initializer {
                        self.lower_expression(init_expr, statements)
                    } else {
                        Operand::Constant(Constant::Null)
                    };

                    let symbols = self.collect_declaration_symbols(binding);
                    for symbol in symbols {
                        let ty = self
                            .builder
                            .type_result
                            .layer()
                            .symbol_type(symbol)
                            .unwrap_or_else(|| TypeId::new(0));

                        let casted_operand = if let Some(init_expr) = initializer {
                            let init_ty = self
                                .builder
                                .type_result
                                .layer()
                                .node_type(init_expr)
                                .unwrap_or_else(|| TypeId::new(0));
                            self.insert_cast_if_needed(operand.clone(), init_ty, ty)
                        } else {
                            operand.clone()
                        };

                        let local_id = self.declare_local(Some(symbol), ty);
                        self.current_instructions
                            .push(Instruction::Assign(local_id, RValue::Use(casted_operand)));
                    }
                }
            }

            SyntaxNodeKind::AssignmentStatement => {
                let target = node.child(0);
                let value = node.child(2);

                if let (Some(target), Some(value)) = (target, value) {
                    if self.lower_index_assignment(target, value, statements) {
                        return;
                    }

                    let operand = self.lower_expression(value, statements);
                    let target_ty = self
                        .builder
                        .type_result
                        .layer()
                        .node_type(target)
                        .unwrap_or_else(|| TypeId::new(0));
                    let value_ty = self
                        .builder
                        .type_result
                        .layer()
                        .node_type(value)
                        .unwrap_or_else(|| TypeId::new(0));
                    let casted_operand = self.insert_cast_if_needed(operand, value_ty, target_ty);

                    if syntax
                        .node(target)
                        .is_some_and(|n| n.kind() == SyntaxNodeKind::NameExpression)
                    {
                        let symbol = resolution.and_then(|res| {
                            res.reference_symbol(target).or_else(|| {
                                let ident = syntax
                                    .first_child_of_kind(target, SyntaxNodeKind::Identifier)?;
                                res.reference_symbol(ident)
                            })
                        });
                        if let Some(sym) = symbol {
                            if let Some(local_id) = self.symbol_to_local.get(&sym).copied() {
                                self.current_instructions.push(Instruction::Assign(
                                    local_id,
                                    RValue::Use(casted_operand),
                                ));
                            } else {
                                let is_global = resolution.is_some_and(|res| {
                                    matches!(
                                        res.symbol(sym).map(|s| s.kind()),
                                        Some(galfus_frontend::SymbolKind::Var)
                                            | Some(galfus_frontend::SymbolKind::Const)
                                    )
                                });
                                if is_global {
                                    let name = resolution
                                        .and_then(|res| res.symbol(sym))
                                        .map(|s| s.name().to_string())
                                        .unwrap_or_default();
                                    self.current_instructions
                                        .push(Instruction::StoreGlobal(name, casted_operand));
                                }
                            }
                        }
                    }
                }
            }

            SyntaxNodeKind::ReturnStatement => {
                let expr = node.first_child();
                let operand = expr.map(|e| {
                    let op = self.lower_expression(e, statements);
                    let expr_ty = self
                        .builder
                        .type_result
                        .layer()
                        .node_type(e)
                        .unwrap_or_else(|| TypeId::new(0));
                    self.insert_cast_if_needed(op, expr_ty, self.return_type)
                });

                let ret_local = match operand.as_ref() {
                    Some(Operand::Local(local_id)) => Some(*local_id),
                    _ => None,
                };

                for scope in self.scopes.iter().rev() {
                    for &local_id in scope.iter().rev() {
                        if Some(local_id) == ret_local {
                            continue;
                        }
                        if let Some(decl) = self.locals.iter().find(|l| l.id == local_id)
                            && self.builder.is_owned_type(decl.ty)
                        {
                            self.current_instructions.push(Instruction::Drop(local_id));
                        }
                    }
                }

                let instructions = std::mem::take(&mut self.current_instructions);
                let block_id = self.builder.next_block();
                statements.push(MirBody::BasicBlock(BasicBlock {
                    id: block_id,
                    instructions,
                    terminator: Terminator::Return(operand),
                }));
            }

            SyntaxNodeKind::BreakStatement => {
                self.flush_current_instructions(statements);
                let block_id = self.builder.next_block();
                statements.push(MirBody::BasicBlock(BasicBlock {
                    id: block_id,
                    instructions: Vec::new(),
                    terminator: Terminator::Break,
                }));
            }

            SyntaxNodeKind::ContinueStatement => {
                self.flush_current_instructions(statements);
                let block_id = self.builder.next_block();
                statements.push(MirBody::BasicBlock(BasicBlock {
                    id: block_id,
                    instructions: Vec::new(),
                    terminator: Terminator::Continue,
                }));
            }

            SyntaxNodeKind::ExpressionStatement => {
                if let Some(expr) = node.first_child() {
                    self.lower_expression(expr, statements);
                }
            }

            SyntaxNodeKind::Block => {
                self.flush_current_instructions(statements);
                let nested_block = self.lower_block(stmt_id);
                statements.push(nested_block);
            }

            SyntaxNodeKind::IfStatement => {
                self.flush_current_instructions(statements);

                let cond_node = node.child(0).unwrap();
                let then_node = node.child(1).unwrap();
                let else_clause_node = node.child(2);

                let cond = self.lower_expression(cond_node, statements);

                self.flush_current_instructions(statements);

                let then_branch = Box::new(self.lower_block(then_node));
                let else_branch = else_clause_node.and_then(|else_clause| {
                    let clause_node = syntax.node(else_clause)?;
                    let child_node = clause_node.first_child()?;
                    Some(Box::new(self.lower_block(child_node)))
                });

                statements.push(MirBody::If {
                    cond,
                    then_branch,
                    else_branch,
                });
            }

            SyntaxNodeKind::WhileStatement => {
                self.flush_current_instructions(statements);

                let cond_node = node.child(0).unwrap();
                let body_node = node.child(1).unwrap();

                let mut loop_body_statements = Vec::new();

                let cond = self.lower_expression(cond_node, &mut loop_body_statements);

                let bool_ty = self
                    .builder
                    .type_result
                    .layer()
                    .node_type(cond_node)
                    .unwrap_or_else(|| TypeId::new(0));

                let temp_id = self.declare_local(None, bool_ty);
                self.current_instructions.push(Instruction::Assign(
                    temp_id,
                    RValue::UnaryOp(MirUnaryOp::Not, cond),
                ));
                let not_cond = Operand::Local(temp_id);

                if !self.current_instructions.is_empty() {
                    let instructions = std::mem::take(&mut self.current_instructions);
                    loop_body_statements.push(MirBody::BasicBlock(BasicBlock {
                        id: self.builder.next_block(),
                        instructions,
                        terminator: Terminator::None,
                    }));
                }

                let break_bb = MirBody::BasicBlock(BasicBlock {
                    id: self.builder.next_block(),
                    instructions: Vec::new(),
                    terminator: Terminator::Break,
                });
                loop_body_statements.push(MirBody::If {
                    cond: not_cond,
                    then_branch: Box::new(break_bb),
                    else_branch: None,
                });

                let lowered_body = self.lower_block(body_node);
                loop_body_statements.push(lowered_body);

                statements.push(MirBody::Loop {
                    body: Box::new(MirBody::Block {
                        locals: Vec::new(),
                        statements: loop_body_statements,
                    }),
                });
            }

            SyntaxNodeKind::LoopStatement => {
                self.flush_current_instructions(statements);

                let body_node = node.child(0).unwrap();
                let body = Box::new(self.lower_block(body_node));

                statements.push(MirBody::Loop { body });
            }

            SyntaxNodeKind::ForStatement => {
                self.flush_current_instructions(statements);

                let binding_node = node.child(0).unwrap();
                let iterable_node = node.child(1).unwrap();
                let body_node = node.child(2).unwrap();

                let iterable_syntax = syntax.node(iterable_node).unwrap();

                if iterable_syntax.kind() == SyntaxNodeKind::RangeExpression {
                    let start_node = iterable_syntax.child(0).unwrap();
                    let end_node = iterable_syntax.child(2).unwrap();
                    let step_node = iterable_syntax.child(3);

                    // Declare the loop variable `binding`
                    let symbols = self.collect_declaration_symbols(binding_node);
                    let symbol = symbols.first().copied();
                    let binding_local = if let Some(sym) = symbol {
                        let ty = self
                            .builder
                            .type_result
                            .layer()
                            .symbol_type(sym)
                            .unwrap_or_else(|| TypeId::new(0));
                        self.declare_local(Some(sym), ty)
                    } else {
                        return;
                    };
                    let binding_type = self
                        .builder
                        .type_result
                        .layer()
                        .symbol_type(symbol.unwrap())
                        .unwrap();

                    // Evaluate and assign `start` to `binding`
                    let start_operand = self.lower_expression(start_node, statements);
                    self.current_instructions.push(Instruction::Assign(
                        binding_local,
                        RValue::Use(start_operand),
                    ));
                    self.flush_current_instructions(statements);

                    // Evaluate and store `end` in a temporary local variable
                    let end_operand = self.lower_expression(end_node, statements);
                    let end_ty = self
                        .builder
                        .type_result
                        .layer()
                        .node_type(end_node)
                        .unwrap_or_else(|| TypeId::new(0));
                    let end_local = self.declare_local(None, end_ty);
                    self.current_instructions
                        .push(Instruction::Assign(end_local, RValue::Use(end_operand)));
                    self.flush_current_instructions(statements);

                    // Evaluate and store `step` in a temporary local variable
                    let step_operand = if let Some(step_wrapper) = step_node {
                        let step_expr_node =
                            syntax.node(step_wrapper).unwrap().first_child().unwrap();
                        self.lower_expression(step_expr_node, statements)
                    } else {
                        Operand::Constant(Constant::Int(1))
                    };
                    let step_ty = binding_type;
                    let step_local = self.declare_local(None, step_ty);
                    self.current_instructions
                        .push(Instruction::Assign(step_local, RValue::Use(step_operand)));
                    self.flush_current_instructions(statements);

                    // Compile the loop body statements
                    let mut loop_body_statements = Vec::new();

                    let range_op_node = iterable_syntax.child(1).unwrap();
                    let range_op_node_data = syntax.node(range_op_node).unwrap();
                    let op_kind = range_op_node_data.range_operator();

                    let is_quantity =
                        matches!(op_kind, Some(galfus_frontend::RangeOperatorKind::Quantity));

                    let counter_local = if is_quantity {
                        let counter_local = self.declare_local(None, end_ty);
                        self.current_instructions.push(Instruction::Assign(
                            counter_local,
                            RValue::Use(Operand::Constant(Constant::Int(0))),
                        ));
                        self.flush_current_instructions(statements);
                        Some(counter_local)
                    } else {
                        None
                    };

                    let bool_type_id = self
                        .builder
                        .type_result
                        .layer()
                        .table()
                        .primitive(PrimitiveType::Bool);

                    let cond_local = self.declare_local(None, bool_type_id);
                    if let Some(counter) = counter_local {
                        self.current_instructions.push(Instruction::Assign(
                            cond_local,
                            RValue::BinaryOp(
                                MirBinaryOp::Less,
                                Operand::Local(counter),
                                Operand::Local(end_local),
                            ),
                        ));
                    } else {
                        let cond_op = match op_kind {
                            Some(galfus_frontend::RangeOperatorKind::Exclusive) | None => {
                                MirBinaryOp::Less
                            }
                            _ => MirBinaryOp::Less,
                        };
                        self.current_instructions.push(Instruction::Assign(
                            cond_local,
                            RValue::BinaryOp(
                                cond_op,
                                Operand::Local(binding_local),
                                Operand::Local(end_local),
                            ),
                        ));
                    }

                    let not_cond_local = self.declare_local(None, bool_type_id);
                    self.current_instructions.push(Instruction::Assign(
                        not_cond_local,
                        RValue::UnaryOp(MirUnaryOp::Not, Operand::Local(cond_local)),
                    ));

                    if !self.current_instructions.is_empty() {
                        let instructions = std::mem::take(&mut self.current_instructions);
                        loop_body_statements.push(MirBody::BasicBlock(BasicBlock {
                            id: self.builder.next_block(),
                            instructions,
                            terminator: Terminator::None,
                        }));
                    }

                    // Break if !condition
                    let break_bb = MirBody::BasicBlock(BasicBlock {
                        id: self.builder.next_block(),
                        instructions: Vec::new(),
                        terminator: Terminator::Break,
                    });
                    loop_body_statements.push(MirBody::If {
                        cond: Operand::Local(not_cond_local),
                        then_branch: Box::new(break_bb),
                        else_branch: None,
                    });

                    // Loop body
                    let lowered_body = self.lower_block(body_node);
                    loop_body_statements.push(lowered_body);

                    // Increment: binding = binding + step
                    let increment_block_id = self.builder.next_block();
                    let mut increment_instructions = vec![Instruction::Assign(
                        binding_local,
                        RValue::BinaryOp(
                            MirBinaryOp::Add,
                            Operand::Local(binding_local),
                            Operand::Local(step_local),
                        ),
                    )];
                    if let Some(counter) = counter_local {
                        increment_instructions.push(Instruction::Assign(
                            counter,
                            RValue::BinaryOp(
                                MirBinaryOp::Add,
                                Operand::Local(counter),
                                Operand::Constant(Constant::Int(1)),
                            ),
                        ));
                    }
                    loop_body_statements.push(MirBody::BasicBlock(BasicBlock {
                        id: increment_block_id,
                        instructions: increment_instructions,
                        terminator: Terminator::None,
                    }));

                    statements.push(MirBody::Loop {
                        body: Box::new(MirBody::Block {
                            locals: Vec::new(),
                            statements: loop_body_statements,
                        }),
                    });
                } else {
                    // Evaluate the iterable expression and store it in a local variable
                    let iterable_operand = self.lower_expression(iterable_node, statements);
                    let iterable_ty = self
                        .builder
                        .type_result
                        .layer()
                        .node_type(iterable_node)
                        .unwrap_or_else(|| TypeId::new(0));
                    let iterable_local = self.declare_local(None, iterable_ty);
                    self.current_instructions.push(Instruction::Assign(
                        iterable_local,
                        RValue::Use(iterable_operand),
                    ));
                    self.flush_current_instructions(statements);

                    // 1. Declare an index counter: let index = 0
                    let int_type_id = self
                        .builder
                        .type_result
                        .layer()
                        .table()
                        .primitive(PrimitiveType::Int32);
                    let index_local = self.declare_local(None, int_type_id);
                    self.current_instructions.push(Instruction::Assign(
                        index_local,
                        RValue::Use(Operand::Constant(Constant::Int(0))),
                    ));
                    self.flush_current_instructions(statements);

                    // 2. Declare a length variable: let len = len(iterable)
                    let len_local = self.declare_local(None, int_type_id);
                    self.current_instructions.push(Instruction::Assign(
                        len_local,
                        RValue::Len(Operand::Local(iterable_local)),
                    ));
                    self.flush_current_instructions(statements);

                    // Cast length to Int32 since index_local is Int32
                    let len_cast_local = self.declare_local(None, int_type_id);
                    self.current_instructions.push(Instruction::Assign(
                        len_cast_local,
                        RValue::Cast(Operand::Local(len_local), int_type_id),
                    ));
                    self.flush_current_instructions(statements);

                    // 3. Declare the loop variable `binding`
                    let symbols = self.collect_declaration_symbols(binding_node);
                    let symbol = symbols.first().copied();
                    let binding_local = if let Some(sym) = symbol {
                        let ty = self
                            .builder
                            .type_result
                            .layer()
                            .symbol_type(sym)
                            .unwrap_or_else(|| TypeId::new(0));
                        self.declare_local(Some(sym), ty)
                    } else {
                        return;
                    };

                    // Compile the loop body statements
                    let mut loop_body_statements = Vec::new();

                    // 4. Condition check: index < len
                    let bool_type_id = self
                        .builder
                        .type_result
                        .layer()
                        .table()
                        .primitive(PrimitiveType::Bool);
                    let cond_local = self.declare_local(None, bool_type_id);
                    self.current_instructions.push(Instruction::Assign(
                        cond_local,
                        RValue::BinaryOp(
                            MirBinaryOp::Less,
                            Operand::Local(index_local),
                            Operand::Local(len_cast_local),
                        ),
                    ));

                    let not_cond_local = self.declare_local(None, bool_type_id);
                    self.current_instructions.push(Instruction::Assign(
                        not_cond_local,
                        RValue::UnaryOp(MirUnaryOp::Not, Operand::Local(cond_local)),
                    ));

                    if !self.current_instructions.is_empty() {
                        let instructions = std::mem::take(&mut self.current_instructions);
                        loop_body_statements.push(MirBody::BasicBlock(BasicBlock {
                            id: self.builder.next_block(),
                            instructions,
                            terminator: Terminator::None,
                        }));
                    }

                    // Break if !condition
                    let break_bb = MirBody::BasicBlock(BasicBlock {
                        id: self.builder.next_block(),
                        instructions: Vec::new(),
                        terminator: Terminator::Break,
                    });
                    loop_body_statements.push(MirBody::If {
                        cond: Operand::Local(not_cond_local),
                        then_branch: Box::new(break_bb),
                        else_branch: None,
                    });

                    // 5. Load the element: binding = iterable[index]
                    let element_instructions_block_id = self.builder.next_block();
                    let element_instructions = vec![Instruction::Assign(
                        binding_local,
                        RValue::ArrayIndex(
                            Operand::Local(iterable_local),
                            Operand::Local(index_local),
                        ),
                    )];
                    loop_body_statements.push(MirBody::BasicBlock(BasicBlock {
                        id: element_instructions_block_id,
                        instructions: element_instructions,
                        terminator: Terminator::None,
                    }));

                    // Loop body
                    let lowered_body = self.lower_block(body_node);
                    loop_body_statements.push(lowered_body);

                    // 6. Increment index: index = index + 1
                    let increment_block_id = self.builder.next_block();
                    let increment_instructions = vec![Instruction::Assign(
                        index_local,
                        RValue::BinaryOp(
                            MirBinaryOp::Add,
                            Operand::Local(index_local),
                            Operand::Constant(Constant::Int(1)),
                        ),
                    )];
                    loop_body_statements.push(MirBody::BasicBlock(BasicBlock {
                        id: increment_block_id,
                        instructions: increment_instructions,
                        terminator: Terminator::None,
                    }));

                    statements.push(MirBody::Loop {
                        body: Box::new(MirBody::Block {
                            locals: Vec::new(),
                            statements: loop_body_statements,
                        }),
                    });
                }
            }

            _ => {}
        }
    }
}
