use crate::mir::*;
use galfus_core::{FunctionId, NodeId, TypeId};
use galfus_frontend::{ModuleGraph, SyntaxNodeKind, TypeCheckResult};

pub struct MirBuilder<'a> {
    graph: &'a ModuleGraph,
    type_result: &'a TypeCheckResult,
    source_text: &'a str,
    next_local_id: u32,
    next_block_id: u32,
}

impl<'a> MirBuilder<'a> {
    pub fn new(
        graph: &'a ModuleGraph,
        type_result: &'a TypeCheckResult,
        source_text: &'a str,
    ) -> Self {
        Self {
            graph,
            type_result,
            source_text,
            next_local_id: 0,
            next_block_id: 0,
        }
    }

    pub fn build(mut self) -> MirModule {
        let mut functions = Vec::new();

        if let Some(root_node) = self
            .graph
            .syntax()
            .root()
            .and_then(|root| self.graph.syntax().node(root))
        {
            for item in root_node.children() {
                if let Some(node) = self.graph.syntax().node(*item) {
                    match node.kind() {
                        SyntaxNodeKind::FunctionItem => {
                            if let Some(func) = self.build_function(*item) {
                                functions.push(func);
                            }
                        }
                        SyntaxNodeKind::ExportItem => {
                            if let Some(inner) = node.first_child() {
                                let is_func = self
                                    .graph
                                    .syntax()
                                    .node(inner)
                                    .map(|inner_node| {
                                        inner_node.kind() == SyntaxNodeKind::FunctionItem
                                    })
                                    .unwrap_or(false);
                                let func = if is_func {
                                    self.build_function(inner)
                                } else {
                                    None
                                };
                                if let Some(func) = func {
                                    functions.push(func);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        MirModule { functions }
    }

    fn build_function(&mut self, item: NodeId) -> Option<MirFunction> {
        let syntax = self.graph.syntax();
        let resolution = self.graph.resolution()?;

        // Find the function name
        let name_node = syntax.first_child_of_kind(item, SyntaxNodeKind::Identifier)?;
        let name = self.node_text(name_node).to_string();

        // Get function symbol and type
        let symbol = resolution.declaration_symbol(name_node)?;
        let func_type = self.type_result.layer().symbol_type(symbol)?;
        let func_id = FunctionId::new(symbol.raw());

        // Parameters
        let mut parameter_types = Vec::new();
        let mut param_symbols = Vec::new();
        if let Some(param_list_node) = syntax
            .first_child_of_kind(item, SyntaxNodeKind::ParameterList)
            .and_then(|param_list| syntax.node(param_list))
        {
            for param in param_list_node.children() {
                let param_node = *param;
                let identifier_node = syntax
                    .first_child_of_kind(param_node, SyntaxNodeKind::Identifier)
                    .unwrap_or(param_node);

                let param_symbol = resolution.declaration_symbol(identifier_node);
                let param_ty = param_symbol
                    .and_then(|sym| self.type_result.layer().symbol_type(sym))
                    .or_else(|| self.type_result.layer().node_type(identifier_node))
                    .or_else(|| self.type_result.layer().node_type(param_node));

                if let Some(ty) = param_ty {
                    parameter_types.push(ty);
                    param_symbols.push((param_symbol, ty));
                }
            }
        }

        // Return Type derived from function signature in the TypeTable
        let return_type = match self.type_result.layer().table().kind(func_type) {
            Some(galfus_frontend::TypeKind::Function(f)) => f.return_type(),
            _ => func_type,
        };

        // Reset the local ID counter for this function
        self.next_local_id = 0;

        let mut builder_ctx = FunctionBuilder {
            builder: self,
            locals: Vec::new(),
            symbol_to_local: std::collections::HashMap::new(),
            current_instructions: Vec::new(),
        };

        // Declare parameters as locals
        for (symbol, ty) in param_symbols {
            builder_ctx.declare_local(symbol, ty);
        }

        // Look for the block of the function body
        let body =
            if let Some(block_node_id) = syntax.first_child_of_kind(item, SyntaxNodeKind::Block) {
                builder_ctx.lower_block(block_node_id)
            } else {
                MirBody::BasicBlock(BasicBlock {
                    id: builder_ctx.builder.next_block(),
                    instructions: Vec::new(),
                    terminator: Terminator::Return(None),
                })
            };

        Some(MirFunction {
            id: func_id,
            name,
            return_type,
            parameter_types,
            locals: builder_ctx.locals,
            body,
        })
    }

    pub fn next_local(&mut self) -> LocalId {
        let id = self.next_local_id;
        self.next_local_id += 1;
        LocalId::new(id)
    }

    pub fn next_block(&mut self) -> BlockId {
        let id = self.next_block_id;
        self.next_block_id += 1;
        BlockId::new(id)
    }

    fn node_text(&self, node: NodeId) -> &str {
        if let Some(syntax_node) = self.graph.syntax().node(node) {
            let span = syntax_node.span();
            if span.start() as usize <= self.source_text.len()
                && span.end() as usize <= self.source_text.len()
            {
                return &self.source_text[span.start() as usize..span.end() as usize];
            }
        }
        ""
    }
}

struct FunctionBuilder<'b, 'a> {
    builder: &'b mut MirBuilder<'a>,
    locals: Vec<LocalDecl>,
    symbol_to_local: std::collections::HashMap<galfus_core::SymbolId, LocalId>,
    current_instructions: Vec<Instruction>,
}

impl<'b, 'a> FunctionBuilder<'b, 'a> {
    fn declare_local(&mut self, symbol: Option<galfus_core::SymbolId>, ty: TypeId) -> LocalId {
        let local_id = self.builder.next_local();
        self.locals.push(LocalDecl { id: local_id, ty });
        if let Some(sym) = symbol {
            self.symbol_to_local.insert(sym, local_id);
        }
        local_id
    }

    fn collect_declaration_symbols(&self, node_id: NodeId) -> Vec<galfus_core::SymbolId> {
        let mut symbols = Vec::new();
        self.collect_symbols_recursive(node_id, &mut symbols);
        symbols
    }

    fn collect_symbols_recursive(&self, node_id: NodeId, symbols: &mut Vec<galfus_core::SymbolId>) {
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

    fn lower_block(&mut self, block_node_id: NodeId) -> MirBody {
        let syntax = self.builder.graph.syntax();
        let Some(block_node) = syntax.node(block_node_id) else {
            return MirBody::BasicBlock(BasicBlock {
                id: self.builder.next_block(),
                instructions: Vec::new(),
                terminator: Terminator::Return(None),
            });
        };

        let mut statements = Vec::new();

        for &stmt_id in block_node.children() {
            self.lower_statement(stmt_id, &mut statements);
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

    fn flush_current_instructions(&mut self, statements: &mut Vec<MirBody>) {
        if !self.current_instructions.is_empty() {
            let instructions = std::mem::take(&mut self.current_instructions);
            statements.push(MirBody::BasicBlock(BasicBlock {
                id: self.builder.next_block(),
                instructions,
                terminator: Terminator::Return(None),
            }));
        }
    }

    fn lower_statement(&mut self, stmt_id: NodeId, statements: &mut Vec<MirBody>) {
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
                        if let Some(local_id) =
                            symbol.and_then(|sym| self.symbol_to_local.get(&sym).copied())
                        {
                            self.current_instructions
                                .push(Instruction::Assign(local_id, RValue::Use(operand)));
                        }
                    }
                }
            }

            SyntaxNodeKind::ReturnStatement => {
                let expr = node.first_child();
                let operand = expr.map(|e| self.lower_expression(e, statements));

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

    fn lower_expression(&mut self, expr_id: NodeId, statements: &mut Vec<MirBody>) -> Operand {
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
                    }
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

            _ => Operand::Constant(Constant::Null),
        }
    }

    fn lower_binary_op(&self, op_node_id: NodeId) -> MirBinaryOp {
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

    fn lower_unary_op(&self, op_node_id: NodeId) -> MirUnaryOp {
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
}

fn parse_int(text: &str) -> Option<i64> {
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
