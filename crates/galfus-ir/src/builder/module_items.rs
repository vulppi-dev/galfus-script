use super::*;
use std::collections::HashMap;

impl<'a> MirBuilder<'a> {
    pub(super) fn build_function(&mut self, item: NodeId) -> Option<MirFunction> {
        self.build_function_with_substitutions(item, None, HashMap::new())
    }

    pub(super) fn build_function_with_substitutions(
        &mut self,
        item: NodeId,
        specialized_id: Option<FunctionId>,
        type_substitutions: HashMap<SymbolId, TypeId>,
    ) -> Option<MirFunction> {
        let syntax = self.graph.syntax();
        let resolution = self.graph.resolution()?;

        // Find the function name
        let name_node = syntax.first_child_of_kind(item, SyntaxNodeKind::Identifier)?;
        let name = self.node_text(name_node).to_string();

        // Get function symbol and type
        let symbol = resolution.declaration_symbol(name_node)?;
        let func_type = self.type_result.layer().symbol_type(symbol)?;
        let func_id = specialized_id.unwrap_or_else(|| FunctionId::new(symbol.raw()));

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
                    let ty = self.substitute_type(ty, &type_substitutions);
                    parameter_types.push(ty);
                    param_symbols.push((param_symbol, ty));
                }
            }
        }

        // Return Type derived from function signature in the TypeTable
        let return_type = match self.type_result.layer().table().kind(func_type) {
            Some(TypeKind::Function(f)) => {
                self.substitute_type(f.return_type(), &type_substitutions)
            }
            _ => func_type,
        };

        // Reset the local ID counter for this function
        self.next_local_id = 0;

        let mut builder_ctx = function::FunctionBuilder {
            builder: self,
            locals: Vec::new(),
            symbol_to_local: std::collections::HashMap::new(),
            current_instructions: Vec::new(),
            scopes: vec![Vec::new()],
            return_type,
            type_substitutions,
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

    pub(super) fn function_item_for_symbol(&self, symbol: SymbolId) -> Option<NodeId> {
        let root = self.graph.syntax().root()?;
        self.find_function_item_for_symbol(root, symbol)
    }

    fn find_function_item_for_symbol(&self, node: NodeId, symbol: SymbolId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        if syntax_node.kind() == SyntaxNodeKind::FunctionItem {
            let name_node = self
                .graph
                .syntax()
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)?;
            if self
                .graph
                .resolution()
                .and_then(|res| res.declaration_symbol(name_node))
                == Some(symbol)
            {
                return Some(node);
            }
        }

        for child in syntax_node.children() {
            if let Some(found) = self.find_function_item_for_symbol(*child, symbol) {
                return Some(found);
            }
        }

        None
    }

    pub(super) fn generic_parameters_for_function_item(&self, item: NodeId) -> Vec<SymbolId> {
        let syntax = self.graph.syntax();
        let Some(generic_list) =
            syntax.first_child_of_kind(item, SyntaxNodeKind::GenericParameterList)
        else {
            return Vec::new();
        };

        let Some(generic_node) = syntax.node(generic_list) else {
            return Vec::new();
        };

        generic_node
            .children()
            .iter()
            .filter_map(|parameter| {
                let identifier =
                    syntax.first_child_of_kind(*parameter, SyntaxNodeKind::Identifier)?;
                self.graph
                    .resolution()
                    .and_then(|res| res.declaration_symbol(identifier))
            })
            .collect()
    }

    pub(super) fn substitute_type(
        &self,
        ty: TypeId,
        substitutions: &std::collections::HashMap<SymbolId, TypeId>,
    ) -> TypeId {
        let ty = self.resolve_alias_type(ty);

        match self.type_result.layer().table().kind(ty) {
            Some(TypeKind::GenericParameter { symbol }) => {
                substitutions.get(symbol).copied().unwrap_or(ty)
            }
            _ => ty,
        }
    }

    pub(super) fn next_specialized_function_id(&mut self) -> FunctionId {
        let id = self.next_specialized_function_id;
        self.next_specialized_function_id = self.next_specialized_function_id.saturating_sub(1);
        FunctionId::new(id)
    }

    pub(super) fn next_local(&mut self) -> LocalId {
        let id = self.next_local_id;
        self.next_local_id += 1;
        LocalId::new(id)
    }

    pub(super) fn next_block(&mut self) -> BlockId {
        let id = self.next_block_id;
        self.next_block_id += 1;
        BlockId::new(id)
    }

    pub(super) fn node_text(&self, node: NodeId) -> &str {
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

    pub(super) fn get_struct_fields(&self, struct_symbol: SymbolId) -> Vec<(String, TypeId)> {
        let mut visited = std::collections::HashSet::new();
        self.get_struct_fields_internal(struct_symbol, &mut visited)
    }

    pub(super) fn get_struct_fields_internal(
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

        let mut fields = Vec::new();
        let root = self.graph.syntax().root().unwrap();
        if let Some(item_node) = self.find_struct_item_by_name(root, struct_symbol_data.name()) {
            let syntax = self.graph.syntax();
            let field_children = syntax
                .first_child_of_kind(item_node, SyntaxNodeKind::StructFieldList)
                .and_then(|fl| syntax.node(fl))
                .map(|n| n.children())
                .unwrap_or(&[]);

            for &field_child in field_children {
                let node_kind = syntax.node(field_child).map(|n| n.kind());
                if node_kind == Some(SyntaxNodeKind::StructExpansion) {
                    let target_sym = syntax
                        .child(field_child, 0)
                        .and_then(|target| self.type_result.layer().node_type(target))
                        .and_then(|target_ty| self.struct_symbol_for_type(target_ty));
                    if let Some(target_sym) = target_sym {
                        for (exp_name, exp_ty) in
                            self.get_struct_fields_internal(target_sym, visited)
                        {
                            if !fields.iter().any(|(n, _)| *n == exp_name) {
                                fields.push((exp_name, exp_ty));
                            }
                        }
                    }
                } else if node_kind == Some(SyntaxNodeKind::StructField)
                    && let Some(ident_node) =
                        syntax.first_child_of_kind(field_child, SyntaxNodeKind::Identifier)
                {
                    let name_str = self.node_text(ident_node).to_string();
                    let field_ty = resolution
                        .declaration_symbol(ident_node)
                        .and_then(|sym| self.type_result.layer().symbol_type(sym))
                        .or_else(|| self.type_result.layer().node_type(field_child));
                    if let Some(ty) = field_ty
                        && !fields.iter().any(|(n, _)| *n == name_str)
                    {
                        fields.push((name_str, ty));
                    }
                }
            }
        }
        fields
    }

    pub(super) fn find_struct_item_by_name(
        &self,
        node: NodeId,
        struct_name: &str,
    ) -> Option<NodeId> {
        let syntax = self.graph.syntax();
        let syntax_node = syntax.node(node)?;
        if syntax_node.kind() == SyntaxNodeKind::StructItem {
            let has_matching_identifier = syntax
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)
                .is_some_and(|id| self.node_text(id) == struct_name);
            if has_matching_identifier {
                return Some(node);
            }
        }
        for &child in syntax_node.children() {
            if let Some(found) = self.find_struct_item_by_name(child, struct_name) {
                return Some(found);
            }
        }
        None
    }

    pub(super) fn struct_symbol_for_type(&self, ty: TypeId) -> Option<SymbolId> {
        let layer = self.type_result.layer();
        let table = layer.table();
        let mut current = ty;
        loop {
            match table.kind(current) {
                Some(TypeKind::Named { symbol }) => {
                    let resolution = self.graph.resolution()?;
                    let is_struct = resolution
                        .symbol(*symbol)
                        .is_some_and(|sd| sd.kind() == SymbolKind::Struct);
                    if is_struct {
                        return Some(*symbol);
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

    pub(super) fn find_struct_field_default_expr(
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

    pub(super) fn find_struct_field_node_by_name(
        &self,
        node: NodeId,
        field_name: &str,
    ) -> Option<NodeId> {
        let syntax = self.graph.syntax();
        let syntax_node = syntax.node(node)?;
        if matches!(
            syntax_node.kind(),
            SyntaxNodeKind::StructField | SyntaxNodeKind::WeakStructField
        ) {
            let matches_name = syntax
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)
                .is_some_and(|identifier| self.node_text(identifier) == field_name);
            if matches_name {
                return Some(node);
            }
        }
        for &child in syntax_node.children() {
            if let Some(found) = self.find_struct_field_node_by_name(child, field_name) {
                return Some(found);
            }
        }
        None
    }

    pub(super) fn find_descendant_of_kind(
        &self,
        node: NodeId,
        kind: SyntaxNodeKind,
    ) -> Option<NodeId> {
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

    pub(super) fn find_tuple_type(&self, elements: &[TypeId]) -> TypeId {
        let table = self.type_result.layer().table();
        for id in 0..table.len() {
            let ty_id = TypeId::new(id as u32);
            if matches!(table.kind(ty_id), Some(TypeKind::Tuple { elements: existing }) if existing == elements)
            {
                return ty_id;
            }
        }
        TypeId::new(0)
    }
}
