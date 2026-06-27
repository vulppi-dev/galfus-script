use crate::mir::*;
use galfus_core::{FunctionId, NodeId, StorageMetadata, SymbolId, TypeId};
use galfus_frontend::{
    ModuleGraph, PathReferenceKind, SymbolKind, SyntaxNodeKind, TypeCheckResult, TypeKind,
};

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

    fn get_struct_fields(&self, struct_symbol: SymbolId) -> Vec<(String, TypeId)> {
        let mut visited = std::collections::HashSet::new();
        self.get_struct_fields_internal(struct_symbol, &mut visited)
    }

    fn get_struct_fields_internal(
        &self,
        struct_symbol: SymbolId,
        visited: &mut std::collections::HashSet<SymbolId>,
    ) -> Vec<(String, TypeId)> {
        if !visited.insert(struct_symbol) {
            return Vec::new();
        }

        let resolution = match self.graph.resolution() {
            Some(res) => res,
            None => return Vec::new(),
        };

        let struct_symbol_data = match resolution.symbol(struct_symbol) {
            Some(data) => data,
            None => return Vec::new(),
        };

        let root = self.graph.syntax().root().unwrap();
        let struct_item = self.find_struct_item_by_name(root, struct_symbol_data.name());

        let mut fields = Vec::new();

        if let Some(item_node) = struct_item {
            let syntax = self.graph.syntax();
            if let Some(field_list) =
                syntax.first_child_of_kind(item_node, SyntaxNodeKind::StructFieldList)
            {
                if let Some(field_list_node) = syntax.node(field_list) {
                    for &field_child in field_list_node.children() {
                        if let Some(field_node) = syntax.node(field_child) {
                            if field_node.kind() == SyntaxNodeKind::StructExpansion {
                                if let Some(target) = syntax.child(field_child, 0) {
                                    if let Some(target_ty) =
                                        self.type_result.layer().node_type(target)
                                    {
                                        if let Some(target_sym) =
                                            self.struct_symbol_for_type(target_ty)
                                        {
                                            for (exp_name, exp_ty) in
                                                self.get_struct_fields_internal(target_sym, visited)
                                            {
                                                if !fields.iter().any(|(n, _)| *n == exp_name) {
                                                    fields.push((exp_name, exp_ty));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(member_scope) = resolution.member_scope(struct_symbol) {
            if let Some(scope) = resolution.scope(member_scope) {
                for (name, &symbol) in scope.symbols() {
                    if let Some(symbol_data) = resolution.symbol(symbol) {
                        if symbol_data.kind() == SymbolKind::StructField {
                            if let Some(ty) = self.type_result.layer().symbol_type(symbol) {
                                let name_str = name.to_string();
                                if let Some(existing) =
                                    fields.iter_mut().find(|(n, _)| *n == name_str)
                                {
                                    existing.1 = ty;
                                } else {
                                    fields.push((name_str, ty));
                                }
                            }
                        }
                    }
                }
            }
        }

        fields
    }

    fn find_struct_item_by_name(&self, node: NodeId, struct_name: &str) -> Option<NodeId> {
        let syntax = self.graph.syntax();
        let syntax_node = syntax.node(node)?;
        if syntax_node.kind() == SyntaxNodeKind::StructItem {
            if let Some(identifier) = syntax.first_child_of_kind(node, SyntaxNodeKind::Identifier) {
                if self.node_text(identifier) == struct_name {
                    return Some(node);
                }
            }
        }
        for &child in syntax_node.children() {
            if let Some(found) = self.find_struct_item_by_name(child, struct_name) {
                return Some(found);
            }
        }
        None
    }

    fn struct_symbol_for_type(&self, ty: TypeId) -> Option<SymbolId> {
        let layer = self.type_result.layer();
        let table = layer.table();
        let mut current = ty;
        loop {
            match table.kind(current) {
                Some(TypeKind::Named { symbol }) => {
                    let resolution = self.graph.resolution()?;
                    if let Some(sym_data) = resolution.symbol(*symbol) {
                        if sym_data.kind() == SymbolKind::Struct {
                            return Some(*symbol);
                        }
                    }
                    break;
                }
                Some(TypeKind::GenericInstance { base, .. }) => {
                    current = *base;
                }
                _ => break,
            }
        }
        None
    }

    fn find_struct_field_default_expr(
        &self,
        struct_symbol: SymbolId,
        field_name: &str,
    ) -> Option<NodeId> {
        let resolution = self.graph.resolution()?;
        let struct_symbol_data = resolution.symbol(struct_symbol)?;
        let root = self.graph.syntax().root().unwrap();
        let struct_item = self.find_struct_item_by_name(root, struct_symbol_data.name())?;

        let field_node = self.find_struct_field_node_by_name(struct_item, field_name)?;
        let syntax = self.graph.syntax();
        let default_node =
            self.find_descendant_of_kind(field_node, SyntaxNodeKind::StructFieldDefault)?;
        syntax.child(default_node, 0)
    }

    fn find_struct_field_node_by_name(&self, node: NodeId, field_name: &str) -> Option<NodeId> {
        let syntax = self.graph.syntax();
        let syntax_node = syntax.node(node)?;
        if matches!(
            syntax_node.kind(),
            SyntaxNodeKind::StructField | SyntaxNodeKind::WeakStructField
        ) {
            if let Some(identifier) = syntax.first_child_of_kind(node, SyntaxNodeKind::Identifier) {
                if self.node_text(identifier) == field_name {
                    return Some(node);
                }
            }
        }
        for &child in syntax_node.children() {
            if let Some(found) = self.find_struct_field_node_by_name(child, field_name) {
                return Some(found);
            }
        }
        None
    }

    fn find_descendant_of_kind(&self, node: NodeId, kind: SyntaxNodeKind) -> Option<NodeId> {
        let syntax = self.graph.syntax();
        let syntax_node = syntax.node(node)?;
        for &child in syntax_node.children() {
            if let Some(child_node) = syntax.node(child) {
                if child_node.kind() == kind {
                    return Some(child);
                }
                if let Some(found) = self.find_descendant_of_kind(child, kind) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn find_tuple_type(&self, elements: &[TypeId]) -> TypeId {
        let table = self.type_result.layer().table();
        for id in 0..table.len() {
            let ty_id = TypeId::new(id as u32);
            if let Some(TypeKind::Tuple { elements: existing }) = table.kind(ty_id) {
                if existing == elements {
                    return ty_id;
                }
            }
        }
        TypeId::new(0)
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
                if self.is_choice_variant_call_target(target_node) {
                    if let Some((variant_name, owner_type, _payload_types)) =
                        self.get_choice_variant_payload(target_node)
                    {
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
                if let Some(res) = resolution {
                    if let Some(kind) = res.path_reference_kind(expr_id) {
                        match kind {
                            PathReferenceKind::EnumVariant => {
                                if let Some(variant_symbol) = res.path_reference_symbol(expr_id) {
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

                    if let Some(list_id) = fields_list_node {
                        if let Some(list_node) = syntax.node(list_id) {
                            for &child_id in list_node.children() {
                                if let Some(child_node) = syntax.node(child_id) {
                                    match child_node.kind() {
                                        SyntaxNodeKind::StructLiteralField => {
                                            let name_ident = syntax
                                                .first_child_of_kind(
                                                    child_id,
                                                    SyntaxNodeKind::Identifier,
                                                )
                                                .unwrap();
                                            let name =
                                                self.builder.node_text(name_ident).to_string();
                                            let val_expr = child_node.child(1).unwrap();
                                            let op = self.lower_expression(val_expr, statements);
                                            field_values.insert(name, op);
                                        }
                                        SyntaxNodeKind::StructLiteralFieldShorthand => {
                                            let name_ident = child_node.first_child().unwrap();
                                            let name =
                                                self.builder.node_text(name_ident).to_string();
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
                    .unwrap_or_else(|| self.find_tuple_type(&element_types));

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

    fn is_choice_variant_call_target(&self, target: NodeId) -> bool {
        let Some(resolution) = self.builder.graph.resolution() else {
            return false;
        };
        matches!(
            resolution.path_reference_kind(target),
            Some(PathReferenceKind::ChoiceVariant)
        )
    }

    fn get_choice_variant_payload(&self, node: NodeId) -> Option<(String, TypeId, Vec<TypeId>)> {
        let resolution = self.builder.graph.resolution()?;
        let variant_symbol = resolution.path_reference_symbol(node)?;
        let owner_symbol = self.owner_symbol_for_member(variant_symbol, SymbolKind::Choice)?;

        let owner_type = self
            .builder
            .type_result
            .layer()
            .symbol_type(owner_symbol)
            .unwrap_or_else(|| TypeId::new(0));

        let variant_name = resolution.symbol(variant_symbol)?.name().to_string();

        let payload_types = self.choice_variant_payload_types(owner_symbol, variant_symbol);

        Some((variant_name, owner_type, payload_types))
    }

    fn owner_symbol_for_member(
        &self,
        member_symbol: SymbolId,
        owner_kind: SymbolKind,
    ) -> Option<SymbolId> {
        let resolution = self.builder.graph.resolution()?;
        for symbol in resolution.symbols() {
            if symbol.kind() != owner_kind {
                continue;
            }
            if let Some(member_scope) = resolution.member_scope(symbol.id()) {
                if let Some(scope) = resolution.scope(member_scope) {
                    if scope.symbols().values().any(|&sym| sym == member_symbol) {
                        return Some(symbol.id());
                    }
                }
            }
        }
        None
    }

    fn choice_variant_payload_types(
        &self,
        owner_symbol: SymbolId,
        variant_symbol: SymbolId,
    ) -> Vec<TypeId> {
        let resolution = match self.builder.graph.resolution() {
            Some(res) => res,
            None => return Vec::new(),
        };
        let _owner_data = match resolution.symbol(owner_symbol) {
            Some(data) => data,
            None => return Vec::new(),
        };
        let variant_data = match resolution.symbol(variant_symbol) {
            Some(data) => data,
            None => return Vec::new(),
        };
        let root = self.builder.graph.syntax().root().unwrap();
        let choice_item = match self.choice_item_node_for_symbol(root, owner_symbol) {
            Some(node) => node,
            None => return Vec::new(),
        };
        let choice_node = match self.builder.graph.syntax().node(choice_item) {
            Some(node) => node,
            None => return Vec::new(),
        };
        let mut variant_node = None;
        for &child in choice_node.children() {
            if let Some(node) = self.find_choice_variant_node_by_name(child, variant_data.name()) {
                variant_node = Some(node);
                break;
            }
        }
        let variant_node_id = match variant_node {
            Some(id) => id,
            None => return Vec::new(),
        };
        let payload = match self
            .builder
            .find_descendant_of_kind(variant_node_id, SyntaxNodeKind::ChoicePayload)
        {
            Some(id) => id,
            None => return Vec::new(),
        };
        let payload_node = match self.builder.graph.syntax().node(payload) {
            Some(node) => node,
            None => return Vec::new(),
        };
        payload_node
            .children()
            .iter()
            .filter_map(|child| {
                let type_node = self.first_type_child(*child).unwrap_or(*child);
                self.builder.type_result.layer().node_type(type_node)
            })
            .collect()
    }

    fn choice_item_node_for_symbol(&self, node: NodeId, choice_symbol: SymbolId) -> Option<NodeId> {
        let syntax_node = self.builder.graph.syntax().node(node)?;
        if syntax_node.kind() == SyntaxNodeKind::ChoiceItem {
            if let Some(resolution) = self.builder.graph.resolution() {
                if let Some(ident) = self
                    .builder
                    .graph
                    .syntax()
                    .first_child_of_kind(node, SyntaxNodeKind::Identifier)
                {
                    if resolution.declaration_symbol(ident) == Some(choice_symbol) {
                        return Some(node);
                    }
                }
            }
        }
        for &child in syntax_node.children() {
            if let Some(found) = self.choice_item_node_for_symbol(child, choice_symbol) {
                return Some(found);
            }
        }
        None
    }

    fn find_choice_variant_node_by_name(&self, node: NodeId, variant_name: &str) -> Option<NodeId> {
        let syntax = self.builder.graph.syntax();
        let syntax_node = syntax.node(node)?;
        if syntax_node.kind() == SyntaxNodeKind::ChoiceVariant {
            if let Some(ident) = syntax.first_child_of_kind(node, SyntaxNodeKind::Identifier) {
                if self.builder.node_text(ident) == variant_name {
                    return Some(node);
                }
            }
        }
        for &child in syntax_node.children() {
            if let Some(found) = self.find_choice_variant_node_by_name(child, variant_name) {
                return Some(found);
            }
        }
        None
    }

    fn first_type_child(&self, node: NodeId) -> Option<NodeId> {
        let syntax = self.builder.graph.syntax();
        let syntax_node = syntax.node(node)?;
        for &child in syntax_node.children() {
            if let Some(child_node) = syntax.node(child) {
                if self.is_type_node_kind(child_node.kind()) {
                    return Some(child);
                }
            }
            if let Some(found) = self.first_type_child(child) {
                return Some(found);
            }
        }
        None
    }

    fn is_type_node_kind(&self, kind: SyntaxNodeKind) -> bool {
        matches!(
            kind,
            SyntaxNodeKind::TypeNull
                | SyntaxNodeKind::NamedType
                | SyntaxNodeKind::Path
                | SyntaxNodeKind::ArrayType
                | SyntaxNodeKind::FixedArrayType
                | SyntaxNodeKind::TupleType
                | SyntaxNodeKind::GroupedType
                | SyntaxNodeKind::UnionType
                | SyntaxNodeKind::GenericType
                | SyntaxNodeKind::FunctionType
        )
    }

    fn resolve_alias_type(&self, ty: TypeId) -> TypeId {
        let mut visited = Vec::new();
        self.resolve_alias_type_with_visited(ty, &mut visited)
    }

    fn resolve_alias_type_with_visited(&self, ty: TypeId, visited: &mut Vec<SymbolId>) -> TypeId {
        let table = self.builder.type_result.layer().table();
        let Some(TypeKind::Named { symbol }) = table.kind(ty) else {
            return ty;
        };
        let Some(resolution) = self.builder.graph.resolution() else {
            return ty;
        };
        let Some(symbol_data) = resolution.symbol(*symbol) else {
            return ty;
        };
        if symbol_data.kind() != SymbolKind::TypeAlias {
            return ty;
        }
        if visited.contains(symbol) {
            return ty;
        }
        visited.push(*symbol);
        if let Some(underlying_ty) = self.builder.type_result.layer().symbol_type(*symbol) {
            self.resolve_alias_type_with_visited(underlying_ty, visited)
        } else {
            ty
        }
    }

    fn get_enum_variant_value(&self, variant_symbol: SymbolId) -> i64 {
        let resolution = match self.builder.graph.resolution() {
            Some(res) => res,
            None => return 0,
        };
        let enum_symbol = match self.owner_symbol_for_member(variant_symbol, SymbolKind::Enum) {
            Some(sym) => sym,
            None => return 0,
        };
        let root = self.builder.graph.syntax().root().unwrap();
        let enum_item = match self.find_enum_item_node_for_symbol(root, enum_symbol) {
            Some(node) => node,
            None => return 0,
        };
        let mut variants = Vec::new();
        self.collect_enum_variants(enum_item, &mut variants);

        let mut current_value = 0;
        for &variant_node in &variants {
            if let Some(ident) = self
                .builder
                .graph
                .syntax()
                .first_child_of_kind(variant_node, SyntaxNodeKind::Identifier)
            {
                let symbol = resolution.declaration_symbol(ident);
                if let Some(val_node) = self
                    .builder
                    .graph
                    .syntax()
                    .first_child_of_kind(variant_node, SyntaxNodeKind::IntegerLiteral)
                {
                    let text = self.builder.node_text(val_node);
                    current_value = parse_int(text).unwrap_or(current_value);
                }
                if symbol == Some(variant_symbol) {
                    return current_value;
                }
            }
            current_value += 1;
        }
        0
    }

    fn find_enum_item_node_for_symbol(
        &self,
        node: NodeId,
        enum_symbol: SymbolId,
    ) -> Option<NodeId> {
        let syntax_node = self.builder.graph.syntax().node(node)?;
        if syntax_node.kind() == SyntaxNodeKind::EnumItem {
            if let Some(resolution) = self.builder.graph.resolution() {
                if let Some(ident) = self
                    .builder
                    .graph
                    .syntax()
                    .first_child_of_kind(node, SyntaxNodeKind::Identifier)
                {
                    if resolution.declaration_symbol(ident) == Some(enum_symbol) {
                        return Some(node);
                    }
                }
            }
        }
        for &child in syntax_node.children() {
            if let Some(found) = self.find_enum_item_node_for_symbol(child, enum_symbol) {
                return Some(found);
            }
        }
        None
    }

    fn collect_enum_variants(&self, node: NodeId, variants: &mut Vec<NodeId>) {
        let syntax_node = match self.builder.graph.syntax().node(node) {
            Some(n) => n,
            None => return,
        };
        if syntax_node.kind() == SyntaxNodeKind::EnumVariant {
            variants.push(node);
            return;
        }
        for &child in syntax_node.children() {
            self.collect_enum_variants(child, variants);
        }
    }

    fn declaration_symbols_in_node(&self, node: NodeId, kinds: &[SymbolKind]) -> Vec<SymbolId> {
        let mut symbols = self.collect_declaration_symbols(node);
        if let Some(res) = self.builder.graph.resolution() {
            symbols.retain(|&symbol| {
                if let Some(sym_data) = res.symbol(symbol) {
                    kinds.contains(&sym_data.kind())
                } else {
                    false
                }
            });
        }
        symbols
    }

    fn variant_pattern_symbols(&self, pattern: NodeId) -> Option<(SymbolId, SymbolId)> {
        let resolution = self.builder.graph.resolution()?;
        let owner_symbol = resolution.reference_symbol(pattern)?;
        let variant_symbol = resolution.path_reference_symbol(pattern)?;
        Some((owner_symbol, variant_symbol))
    }

    fn lower_match_arms(
        &mut self,
        arms: &[NodeId],
        index: usize,
        subject: &Operand,
        result_local: LocalId,
    ) -> MirBody {
        let syntax = self.builder.graph.syntax();
        if index >= arms.len() {
            let mut insts = Vec::new();
            insts.push(Instruction::Assign(
                result_local,
                RValue::Use(Operand::Constant(Constant::Null)),
            ));
            return MirBody::BasicBlock(BasicBlock {
                id: self.builder.next_block(),
                instructions: insts,
                terminator: Terminator::Return(None),
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

    fn lower_pattern_check(
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

                        let mut bind_insts = Vec::new();
                        bind_insts
                            .push(Instruction::Assign(local_id, RValue::Use(subject.clone())));
                        bindings.push(MirBody::BasicBlock(BasicBlock {
                            id: self.builder.next_block(),
                            instructions: bind_insts,
                            terminator: Terminator::Return(None),
                        }));
                    }
                }
                Operand::Constant(Constant::Bool(true))
            }

            SyntaxNodeKind::VariantPattern => {
                if let Some((owner_symbol, variant_symbol)) =
                    self.variant_pattern_symbols(pattern_node_id)
                {
                    if let Some(res) = resolution {
                        if let Some(variant_data) = res.symbol(variant_symbol) {
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
                                        if let Some(TypeKind::Named { symbol }) = table.kind(ty_id)
                                        {
                                            if *symbol == variant_symbol {
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

                                        let payload_types = self.choice_variant_payload_types(
                                            owner_symbol,
                                            variant_symbol,
                                        );

                                        if !payload_patterns.is_empty() {
                                            let payload_ty = if payload_patterns.len() > 1 {
                                                self.builder.find_tuple_type(&payload_types)
                                            } else {
                                                payload_types[0]
                                            };

                                            let payload_temp = self.declare_local(None, payload_ty);
                                            let mut extract_insts = Vec::new();
                                            extract_insts.push(Instruction::Assign(
                                                payload_temp,
                                                RValue::MemberAccess(subject.clone(), variant_name),
                                            ));

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
                                                    terminator: Terminator::Return(None),
                                                }));
                                                bindings.extend(nested_bindings);
                                            } else {
                                                bindings.push(MirBody::BasicBlock(BasicBlock {
                                                    id: self.builder.next_block(),
                                                    instructions: extract_insts,
                                                    terminator: Terminator::Return(None),
                                                }));
                                                for (i, &child_pattern) in
                                                    payload_patterns.iter().enumerate()
                                                {
                                                    let element_ty = payload_types[i];
                                                    let element_temp =
                                                        self.declare_local(None, element_ty);
                                                    let mut elem_insts = Vec::new();
                                                    elem_insts.push(Instruction::Assign(
                                                        element_temp,
                                                        RValue::MemberAccess(
                                                            payload_op.clone(),
                                                            i.to_string(),
                                                        ),
                                                    ));
                                                    bindings.push(MirBody::BasicBlock(
                                                        BasicBlock {
                                                            id: self.builder.next_block(),
                                                            instructions: elem_insts,
                                                            terminator: Terminator::Return(None),
                                                        },
                                                    ));

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

                if let Some(binding_node_id) =
                    syntax.first_child_of_kind(pattern_node_id, SyntaxNodeKind::TypePatternBinding)
                {
                    if resolution.is_some() {
                        let symbols = self.declaration_symbols_in_node(
                            binding_node_id,
                            &[SymbolKind::TypePatternBinding],
                        );
                        for symbol in symbols {
                            let local_id = self.declare_local(Some(symbol), pattern_type);
                            self.symbol_to_local.insert(symbol, local_id);

                            let mut bind_insts = Vec::new();
                            bind_insts
                                .push(Instruction::Assign(local_id, RValue::Use(subject.clone())));
                            bindings.push(MirBody::BasicBlock(BasicBlock {
                                id: self.builder.next_block(),
                                instructions: bind_insts,
                                terminator: Terminator::Return(None),
                            }));
                        }
                    }
                }

                Operand::Local(cond_temp)
            }

            _ => Operand::Constant(Constant::Null),
        }
    }

    fn struct_symbol_for_type(&self, ty: TypeId) -> Option<SymbolId> {
        self.builder.struct_symbol_for_type(ty)
    }

    fn find_struct_field_default_expr(
        &self,
        struct_symbol: SymbolId,
        field_name: &str,
    ) -> Option<NodeId> {
        self.builder
            .find_struct_field_default_expr(struct_symbol, field_name)
    }

    fn get_struct_fields(&self, struct_symbol: SymbolId) -> Vec<(String, TypeId)> {
        self.builder.get_struct_fields(struct_symbol)
    }

    fn find_tuple_type(&self, elements: &[TypeId]) -> TypeId {
        self.builder.find_tuple_type(elements)
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
