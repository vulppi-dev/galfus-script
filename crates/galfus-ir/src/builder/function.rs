use crate::mir::*;
use galfus_core::{NodeId, SymbolId, TypeId};
use galfus_frontend::SyntaxNodeKind;

pub struct FunctionBuilder<'b, 'a> {
    pub(super) builder: &'b mut super::MirBuilder<'a>,
    pub(super) locals: Vec<LocalDecl>,
    pub(super) symbol_to_local: std::collections::HashMap<SymbolId, LocalId>,
    pub(super) current_instructions: Vec<Instruction>,
    pub(super) scopes: Vec<Vec<LocalId>>,
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

                        let local_id = self.declare_local(Some(symbol), ty);
                        self.current_instructions
                            .push(Instruction::Assign(local_id, RValue::Use(operand.clone())));
                    }
                }
            }

            SyntaxNodeKind::AssignmentStatement => {
                let target = node.child(0);
                let value = node.child(2);

                if let (Some(target), Some(value)) = (target, value) {
                    let operand = self.lower_expression(value, statements);

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
                                self.current_instructions
                                    .push(Instruction::Assign(local_id, RValue::Use(operand)));
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
                                        .push(Instruction::StoreGlobal(name, operand));
                                }
                            }
                        }
                    }
                }
            }

            SyntaxNodeKind::ReturnStatement => {
                let expr = node.first_child();
                let operand = expr.map(|e| self.lower_expression(e, statements));

                let ret_local = match &operand {
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
                        terminator: Terminator::Return(None),
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

                if let Some(iterable_node) = node.child(1) {
                    self.lower_expression(iterable_node, statements);
                    self.flush_current_instructions(statements);
                }

                let body_node = node.child(2).unwrap();
                let body = Box::new(self.lower_block(body_node));

                statements.push(MirBody::Loop { body });
            }

            _ => {}
        }
    }

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
