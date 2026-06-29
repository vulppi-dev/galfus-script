use crate::mir::*;
use galfus_core::{FunctionId, NodeId, SymbolId, TypeId};
use galfus_frontend::{ModuleGraph, SymbolKind, SyntaxNodeKind, TypeCheckResult, TypeKind};

pub mod complex_literals;
pub mod expression;
pub mod function;
mod function_helpers;
pub mod helpers;
mod module_items;
pub mod pattern;

pub struct MirBuilder<'a> {
    pub(super) graph: &'a ModuleGraph,
    pub(super) type_result: &'a TypeCheckResult,
    pub(super) source_text: &'a str,
    pub(super) next_local_id: u32,
    pub(super) next_block_id: u32,
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
        let mut globals = Vec::new();
        let mut global_items = Vec::new();

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
                                if is_func {
                                    if let Some(func) = self.build_function(inner) {
                                        functions.push(func);
                                    }
                                } else if let Some(inner_node) = self.graph.syntax().node(inner)
                                    && matches!(
                                        inner_node.kind(),
                                        SyntaxNodeKind::VarItem | SyntaxNodeKind::ConstItem
                                    )
                                {
                                    global_items.push(inner);
                                }
                            }
                        }
                        SyntaxNodeKind::VarItem | SyntaxNodeKind::ConstItem => {
                            global_items.push(*item);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Process global declarations
        for &item in &global_items {
            if let Some(binding) = self
                .graph
                .syntax()
                .first_child_of_kind(item, SyntaxNodeKind::BindingPattern)
            {
                let mut symbols = Vec::new();
                self.collect_symbols_recursive_in_builder(binding, &mut symbols);
                for symbol in symbols {
                    let ty = self
                        .type_result
                        .layer()
                        .symbol_type(symbol)
                        .unwrap_or_else(|| TypeId::new(0));
                    let name = self
                        .graph
                        .resolution()
                        .and_then(|res| res.symbol(symbol))
                        .map(|sym| sym.name().to_string())
                        .unwrap_or_default();
                    globals.push(GlobalDecl { name, ty });
                }
            }
        }

        // Build synthetic init function if there are global initializers
        if let Some(init_func) = self.build_global_initializers(&global_items) {
            functions.push(init_func);
        }

        MirModule { functions, globals }
    }

    fn collect_symbols_recursive_in_builder(&self, node_id: NodeId, symbols: &mut Vec<SymbolId>) {
        if let Some(sym) = self
            .graph
            .resolution()
            .and_then(|res| res.declaration_symbol(node_id))
        {
            symbols.push(sym);
        }
        if let Some(node) = self.graph.syntax().node(node_id) {
            for &child in node.children() {
                self.collect_symbols_recursive_in_builder(child, symbols);
            }
        }
    }

    fn build_global_initializers(&mut self, items: &[NodeId]) -> Option<MirFunction> {
        let mut builder_ctx = function::FunctionBuilder {
            builder: self,
            locals: Vec::new(),
            symbol_to_local: std::collections::HashMap::new(),
            current_instructions: Vec::new(),
            scopes: vec![Vec::new()],
            return_type: TypeId::new(0),
        };

        let mut statements = Vec::new();
        let syntax = builder_ctx.builder.graph.syntax();

        for &item in items {
            if let Some(node) = syntax.node(item)
                && matches!(
                    node.kind(),
                    SyntaxNodeKind::VarItem | SyntaxNodeKind::ConstItem
                )
                && let Some(initializer) =
                    syntax.first_child_of_kind(item, SyntaxNodeKind::Initializer)
                && let Some(expr) = syntax.first_child(initializer)
            {
                let operand = builder_ctx.lower_expression(expr, &mut statements);

                if let Some(binding) =
                    syntax.first_child_of_kind(item, SyntaxNodeKind::BindingPattern)
                {
                    let symbols = builder_ctx.collect_declaration_symbols(binding);
                    for symbol in symbols {
                        let name = builder_ctx
                            .builder
                            .graph
                            .resolution()
                            .and_then(|res| res.symbol(symbol))
                            .map(|sym| sym.name().to_string())
                            .unwrap_or_default();

                        builder_ctx.flush_current_instructions(&mut statements);
                        builder_ctx
                            .current_instructions
                            .push(Instruction::StoreGlobal(name, operand.clone()));
                    }
                }
            }
        }

        builder_ctx.flush_current_instructions(&mut statements);

        if statements.is_empty() {
            return None;
        }

        let body = if statements.len() == 1 {
            statements.pop().unwrap()
        } else {
            MirBody::Block {
                locals: Vec::new(),
                statements,
            }
        };

        Some(MirFunction {
            id: FunctionId::new(u32::MAX),
            name: "__init_module".to_string(),
            return_type: TypeId::new(0),
            parameter_types: Vec::new(),
            locals: builder_ctx.locals,
            body,
        })
    }

    pub(super) fn is_owned_type(&self, ty: TypeId) -> bool {
        let ty = self.resolve_alias_type(ty);
        let table = self.type_result.layer().table();

        match table.kind(ty) {
            Some(TypeKind::Named { symbol }) => self
                .graph
                .resolution()
                .and_then(|resolution| resolution.symbol(*symbol))
                .is_some_and(|symbol| {
                    matches!(symbol.kind(), SymbolKind::Struct | SymbolKind::Choice)
                }),

            Some(TypeKind::Array { .. })
            | Some(TypeKind::FixedArray { .. })
            | Some(TypeKind::Tuple { .. }) => true,

            Some(TypeKind::Union { members }) => members
                .iter()
                .copied()
                .any(|member| self.is_owned_type(member)),

            _ => false,
        }
    }

    pub(super) fn resolve_alias_type(&self, ty: TypeId) -> TypeId {
        let mut visited = Vec::new();
        self.resolve_alias_type_with_visited(ty, &mut visited)
    }

    pub(super) fn resolve_alias_type_with_visited(
        &self,
        ty: TypeId,
        visited: &mut Vec<SymbolId>,
    ) -> TypeId {
        let table = self.type_result.layer().table();
        let Some(TypeKind::Named { symbol }) = table.kind(ty) else {
            return ty;
        };
        let Some(resolution) = self.graph.resolution() else {
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
        let underlying_ty = self.type_result.layer().symbol_type(*symbol).unwrap_or(ty);
        self.resolve_alias_type_with_visited(underlying_ty, visited)
    }

    pub(super) fn is_assignable(&self, expected: TypeId, actual: TypeId) -> bool {
        let expected = self.resolve_alias_type(expected);
        let actual = self.resolve_alias_type(actual);

        if expected == actual {
            return true;
        }

        let table = self.type_result.layer().table();
        let expected_kind = table.kind(expected);
        let actual_kind = table.kind(actual);

        if matches!(expected_kind, Some(TypeKind::Error)) {
            return true;
        }

        if matches!(actual_kind, Some(TypeKind::Error)) {
            return true;
        }

        match (expected_kind, actual_kind) {
            (Some(TypeKind::Union { members }), _) => members
                .iter()
                .copied()
                .any(|member| self.is_assignable(member, actual)),

            (_, Some(TypeKind::Union { members })) => members
                .iter()
                .copied()
                .all(|member| self.is_assignable(expected, member)),

            (
                Some(TypeKind::Array {
                    element: expected_element,
                }),
                Some(TypeKind::Array {
                    element: actual_element,
                }),
            ) => self.is_assignable(*expected_element, *actual_element),

            (
                Some(TypeKind::Array {
                    element: expected_element,
                }),
                Some(TypeKind::FixedArray {
                    element: actual_element,
                    ..
                }),
            ) => self.is_assignable(*expected_element, *actual_element),

            (
                Some(TypeKind::FixedArray {
                    element: expected_element,
                    size: expected_size,
                }),
                Some(TypeKind::FixedArray {
                    element: actual_element,
                    size: actual_size,
                }),
            ) => {
                expected_size == actual_size
                    && self.is_assignable(*expected_element, *actual_element)
            }

            (
                Some(TypeKind::Primitive(expected_primitive)),
                Some(TypeKind::Primitive(actual_primitive)),
            ) => {
                if expected_primitive == actual_primitive {
                    true
                } else {
                    let is_expected_int = matches!(
                        expected_primitive,
                        galfus_frontend::PrimitiveType::Int8
                            | galfus_frontend::PrimitiveType::Int16
                            | galfus_frontend::PrimitiveType::Int32
                            | galfus_frontend::PrimitiveType::Int64
                            | galfus_frontend::PrimitiveType::Uint8
                            | galfus_frontend::PrimitiveType::Uint16
                            | galfus_frontend::PrimitiveType::Uint32
                            | galfus_frontend::PrimitiveType::Uint64
                    );
                    let is_actual_int = matches!(
                        actual_primitive,
                        galfus_frontend::PrimitiveType::Int8
                            | galfus_frontend::PrimitiveType::Int16
                            | galfus_frontend::PrimitiveType::Int32
                            | galfus_frontend::PrimitiveType::Int64
                            | galfus_frontend::PrimitiveType::Uint8
                            | galfus_frontend::PrimitiveType::Uint16
                            | galfus_frontend::PrimitiveType::Uint32
                            | galfus_frontend::PrimitiveType::Uint64
                    );
                    if is_expected_int && is_actual_int {
                        true
                    } else {
                        let is_expected_float = matches!(
                            expected_primitive,
                            galfus_frontend::PrimitiveType::Float16
                                | galfus_frontend::PrimitiveType::Float32
                                | galfus_frontend::PrimitiveType::Float64
                        );
                        let is_actual_float = matches!(
                            actual_primitive,
                            galfus_frontend::PrimitiveType::Float16
                                | galfus_frontend::PrimitiveType::Float32
                                | galfus_frontend::PrimitiveType::Float64
                        );
                        is_expected_float && is_actual_float
                    }
                }
            }

            _ => false,
        }
    }
}
