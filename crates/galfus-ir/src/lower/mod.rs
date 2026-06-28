use galfus_core::{FunctionId, NodeId, SymbolId, TypeId};
use galfus_frontend::{
    ArraySize, ModuleGraph, PrimitiveType, SymbolKind, SyntaxNodeKind, TypeCheckResult, TypeKind,
};
use galfus_image::instruction::{ConstIdx, FuncIdx, TypeIdx};
use galfus_image::*;
use std::collections::{HashMap, HashSet};

pub mod control_flow;
mod expression;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HashableConstant {
    Bool(bool),
    Int(i64),
    FloatBits(u64),
    String(String),
}

impl HashableConstant {
    pub fn from_mir(constant: &crate::mir::Constant) -> Option<Self> {
        match constant {
            crate::mir::Constant::Null => None,
            crate::mir::Constant::Bool(b) => Some(Self::Bool(*b)),
            crate::mir::Constant::Int(i) => Some(Self::Int(*i)),
            crate::mir::Constant::Float(f) => Some(Self::FloatBits(f.to_bits())),
            crate::mir::Constant::String(s) => Some(Self::String(s.clone())),
        }
    }
}

pub struct LowerCtx<'a> {
    pub type_result: &'a TypeCheckResult,
    pub graph: &'a ModuleGraph,
    pub source_text: &'a str,
    pub types: Vec<ImageType>,
    pub struct_layouts: Vec<StructLayout>,
    pub choice_layouts: Vec<ChoiceLayout>,
    pub type_map: HashMap<TypeId, TypeIdx>,
    pub struct_map: HashMap<SymbolId, StructLayoutIdx>,
    pub choice_map: HashMap<SymbolId, ChoiceLayoutIdx>,
    pub constant_pool: ConstantPool,
    pub constants_map: HashMap<HashableConstant, ConstIdx>,
    pub function_map: HashMap<FunctionId, FuncIdx>,
    pub function_names: HashMap<FunctionId, String>,
}

impl<'a> LowerCtx<'a> {
    pub fn new(
        type_result: &'a TypeCheckResult,
        graph: &'a ModuleGraph,
        source_text: &'a str,
    ) -> Self {
        Self {
            type_result,
            graph,
            source_text,
            types: Vec::new(),
            struct_layouts: Vec::new(),
            choice_layouts: Vec::new(),
            type_map: HashMap::new(),
            struct_map: HashMap::new(),
            choice_map: HashMap::new(),
            constant_pool: ConstantPool {
                constants: Vec::new(),
            },
            constants_map: HashMap::new(),
            function_map: HashMap::new(),
            function_names: HashMap::new(),
        }
    }

    pub fn lower_type(&mut self, ty: TypeId) -> TypeIdx {
        let ty = self.resolve_alias_type(ty);
        if let Some(&idx) = self.type_map.get(&ty) {
            return idx;
        }

        let next_idx = TypeIdx(self.types.len() as u16);
        self.type_map.insert(ty, next_idx);
        self.types.push(ImageType::Null);

        let table = self.type_result.layer().table();
        let image_type = match table.kind(ty) {
            Some(TypeKind::Primitive(prim)) => self.lower_primitive(*prim),
            Some(TypeKind::Named { symbol }) => {
                let resolution = self.graph.resolution().unwrap();
                let sym_kind = resolution.symbol(*symbol).map(|s| s.kind());
                match sym_kind {
                    Some(SymbolKind::Struct) => {
                        let layout_idx = self.get_or_create_struct_layout(*symbol);
                        ImageType::Struct(layout_idx)
                    }
                    Some(SymbolKind::Choice) => {
                        let layout_idx = self.get_or_create_choice_layout(*symbol);
                        ImageType::Choice(layout_idx)
                    }
                    _ => ImageType::Null,
                }
            }
            Some(TypeKind::Array { element }) => {
                let elem_idx = self.lower_type(*element);
                ImageType::Array(elem_idx)
            }
            Some(TypeKind::FixedArray { element, size }) => {
                let elem_idx = self.lower_type(*element);
                let len = match size {
                    ArraySize::Known(n) => *n,
                    _ => 0,
                };
                ImageType::FixedArray(elem_idx, len as usize)
            }
            Some(TypeKind::Tuple { elements }) => {
                let elem_idxs = elements.iter().map(|&e| self.lower_type(e)).collect();
                ImageType::Tuple(elem_idxs)
            }
            Some(TypeKind::GenericInstance { base, .. }) => {
                let base_idx = self.lower_type(*base);
                self.types[base_idx.raw() as usize].clone()
            }
            _ => ImageType::Null,
        };

        self.types[next_idx.raw() as usize] = image_type;
        next_idx
    }

    fn lower_primitive(&self, prim: PrimitiveType) -> ImageType {
        match prim {
            PrimitiveType::Null => ImageType::Null,
            PrimitiveType::Bool => ImageType::Bool,
            PrimitiveType::Int8 => ImageType::Int8,
            PrimitiveType::Int16 => ImageType::Int16,
            PrimitiveType::Int32 => ImageType::Int32,
            PrimitiveType::Int64 => ImageType::Int64,
            PrimitiveType::Uint8 => ImageType::Uint8,
            PrimitiveType::Uint16 => ImageType::Uint16,
            PrimitiveType::Uint32 => ImageType::Uint32,
            PrimitiveType::Uint64 => ImageType::Uint64,
            PrimitiveType::Float16 => ImageType::Float32,
            PrimitiveType::Float32 => ImageType::Float32,
            PrimitiveType::Float64 => ImageType::Float64,
        }
    }

    pub fn get_or_create_struct_layout(&mut self, struct_symbol: SymbolId) -> StructLayoutIdx {
        if let Some(&idx) = self.struct_map.get(&struct_symbol) {
            return idx;
        }

        let next_idx = StructLayoutIdx(self.struct_layouts.len() as u16);
        self.struct_map.insert(struct_symbol, next_idx);

        let resolution = self.graph.resolution().unwrap();
        let symbol_data = resolution.symbol(struct_symbol).unwrap();
        let struct_name = symbol_data.name().to_string();

        let raw_fields = self.get_struct_fields(struct_symbol);
        let fields = raw_fields
            .into_iter()
            .map(|(name, ty)| {
                let ty_idx = self.lower_type(ty);
                FieldLayout {
                    name,
                    ty: ty_idx,
                    offset: 0,
                    ownership: OwnershipKind::Value,
                }
            })
            .collect();

        self.struct_layouts.push(StructLayout {
            name: struct_name,
            fields,
        });

        next_idx
    }

    pub fn get_or_create_choice_layout(&mut self, choice_symbol: SymbolId) -> ChoiceLayoutIdx {
        if let Some(&idx) = self.choice_map.get(&choice_symbol) {
            return idx;
        }

        let next_idx = ChoiceLayoutIdx(self.choice_layouts.len() as u16);
        self.choice_map.insert(choice_symbol, next_idx);

        let resolution = self.graph.resolution().unwrap();
        let symbol_data = resolution.symbol(choice_symbol).unwrap();
        let choice_name = symbol_data.name().to_string();

        let raw_variants = self.get_choice_variants(choice_symbol);
        let variants = raw_variants
            .into_iter()
            .map(|(name, payload_ty)| {
                let payload_idx = payload_ty.map(|ty| self.lower_type(ty));
                ChoiceVariantLayout {
                    name,
                    payload_ty: payload_idx,
                }
            })
            .collect();

        self.choice_layouts.push(ChoiceLayout {
            name: choice_name,
            variants,
        });

        next_idx
    }

    pub fn get_or_create_constant(&mut self, constant: &crate::mir::Constant) -> ConstIdx {
        let hashable = match HashableConstant::from_mir(constant) {
            Some(h) => h,
            None => return ConstIdx(0), // Placeholder for Null
        };

        if let Some(&idx) = self.constants_map.get(&hashable) {
            return idx;
        }

        let next_idx = ConstIdx(self.constant_pool.constants.len() as u16);
        self.constants_map.insert(hashable, next_idx);

        let c = match constant {
            crate::mir::Constant::Null => unreachable!(),
            crate::mir::Constant::Bool(b) => Constant::Bool(*b),
            crate::mir::Constant::Int(i) => Constant::Int(*i),
            crate::mir::Constant::Float(f) => Constant::Float(*f),
            crate::mir::Constant::String(s) => Constant::String(s.clone()),
        };

        self.constant_pool.constants.push(c);
        next_idx
    }

    pub fn resolve_alias_type(&self, ty: TypeId) -> TypeId {
        let mut visited = Vec::new();
        self.resolve_alias_type_with_visited(ty, &mut visited)
    }

    pub fn resolve_alias_type_with_visited(
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

    pub fn get_struct_fields(&self, struct_symbol: SymbolId) -> Vec<(String, TypeId)> {
        let mut visited = HashSet::new();
        self.get_struct_fields_internal(struct_symbol, &mut visited)
    }

    fn get_struct_fields_internal(
        &self,
        struct_symbol: SymbolId,
        visited: &mut HashSet<SymbolId>,
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

        if let Some(scope) = resolution
            .member_scope(struct_symbol)
            .and_then(|ms| resolution.scope(ms))
        {
            for (name, &symbol) in scope.symbols() {
                let field_ty = resolution
                    .symbol(symbol)
                    .filter(|sd| sd.kind() == SymbolKind::StructField)
                    .and_then(|_| self.type_result.layer().symbol_type(symbol));
                if let Some(ty) = field_ty {
                    let name_str = name.to_string();
                    if let Some(existing) = fields.iter_mut().find(|(n, _)| *n == name_str) {
                        existing.1 = ty;
                    } else {
                        fields.push((name_str, ty));
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

    pub fn node_text(&self, node: NodeId) -> &str {
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

    fn struct_symbol_for_type(&self, ty: TypeId) -> Option<SymbolId> {
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

    pub fn get_choice_variants(&self, choice_symbol: SymbolId) -> Vec<(String, Option<TypeId>)> {
        let resolution = match self.graph.resolution() {
            Some(res) => res,
            None => return Vec::new(),
        };
        let mut variants = Vec::new();
        let root = self.graph.syntax().root().unwrap();
        if let Some(choice_node_id) = self.choice_item_node_for_symbol(root, choice_symbol) {
            let syntax = self.graph.syntax();
            let variant_list_node = syntax
                .first_child_of_kind(choice_node_id, SyntaxNodeKind::ChoiceVariantList)
                .unwrap_or(choice_node_id);
            if let Some(choice_node) = syntax.node(variant_list_node) {
                for &child in choice_node.children() {
                    if let Some(variant_node) = syntax.node(child)
                        && variant_node.kind() == SyntaxNodeKind::ChoiceVariant
                        && let Some(ident_node) =
                            syntax.first_child_of_kind(child, SyntaxNodeKind::Identifier)
                    {
                        let variant_name = self.node_text(ident_node).to_string();
                        if let Some(variant_symbol) = resolution.declaration_symbol(ident_node) {
                            let payload_types =
                                self.choice_variant_payload_types(choice_symbol, variant_symbol);
                            let payload_ty = if payload_types.is_empty() {
                                None
                            } else if payload_types.len() == 1 {
                                Some(payload_types[0])
                            } else {
                                Some(self.find_tuple_type(&payload_types))
                            };
                            variants.push((variant_name, payload_ty));
                        }
                    }
                }
            }
        }
        variants
    }

    fn choice_item_node_for_symbol(&self, node: NodeId, choice_symbol: SymbolId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;
        if syntax_node.kind() == SyntaxNodeKind::ChoiceItem {
            let matches_symbol = self.graph.resolution().is_some_and(|res| {
                self.graph
                    .syntax()
                    .first_child_of_kind(node, SyntaxNodeKind::Identifier)
                    .and_then(|ident| res.declaration_symbol(ident))
                    == Some(choice_symbol)
            });
            if matches_symbol {
                return Some(node);
            }
        }
        for &child in syntax_node.children() {
            if let Some(found) = self.choice_item_node_for_symbol(child, choice_symbol) {
                return Some(found);
            }
        }
        None
    }

    fn choice_variant_payload_types(
        &self,
        owner_symbol: SymbolId,
        variant_symbol: SymbolId,
    ) -> Vec<TypeId> {
        let resolution = match self.graph.resolution() {
            Some(res) => res,
            None => return Vec::new(),
        };
        let variant_data = match resolution.symbol(variant_symbol) {
            Some(data) => data,
            None => return Vec::new(),
        };
        let root = self.graph.syntax().root().unwrap();
        let choice_item = match self.choice_item_node_for_symbol(root, owner_symbol) {
            Some(node) => node,
            None => return Vec::new(),
        };
        let choice_node = match self.graph.syntax().node(choice_item) {
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
        let payload =
            match self.find_descendant_of_kind(variant_node_id, SyntaxNodeKind::ChoicePayload) {
                Some(id) => id,
                None => return Vec::new(),
            };
        let payload_node = match self.graph.syntax().node(payload) {
            Some(node) => node,
            None => return Vec::new(),
        };
        payload_node
            .children()
            .iter()
            .filter_map(|child| {
                let type_node = self.first_type_child(*child).unwrap_or(*child);
                self.type_result.layer().node_type(type_node)
            })
            .collect()
    }

    fn find_choice_variant_node_by_name(&self, node: NodeId, name: &str) -> Option<NodeId> {
        let syntax = self.graph.syntax();
        let syntax_node = syntax.node(node)?;
        if syntax_node.kind() == SyntaxNodeKind::ChoiceVariant {
            let matches_name = syntax
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)
                .is_some_and(|identifier| self.node_text(identifier) == name);
            if matches_name {
                return Some(node);
            }
        }
        for &child in syntax_node.children() {
            if let Some(found) = self.find_choice_variant_node_by_name(child, name) {
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

    fn first_type_child(&self, node: NodeId) -> Option<NodeId> {
        let syntax = self.graph.syntax();
        let syntax_node = syntax.node(node)?;
        for &child in syntax_node.children() {
            if let Some(child_node) = syntax.node(child)
                && matches!(
                    child_node.kind(),
                    SyntaxNodeKind::NamedType
                        | SyntaxNodeKind::ArrayType
                        | SyntaxNodeKind::FixedArrayType
                        | SyntaxNodeKind::TupleType
                        | SyntaxNodeKind::UnionType
                )
            {
                return Some(child);
            }
        }
        None
    }

    fn find_tuple_type(&self, elements: &[TypeId]) -> TypeId {
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

pub fn lower_module(
    mir_module: &crate::mir::MirModule,
    type_result: &TypeCheckResult,
    module_graph: &ModuleGraph,
    source_text: &str,
) -> ModuleImage {
    let mut ctx = LowerCtx::new(type_result, module_graph, source_text);

    for (i, func) in mir_module.functions.iter().enumerate() {
        ctx.function_map.insert(func.id, FuncIdx(i as u16));
        ctx.function_names.insert(func.id, func.name.clone());
    }

    let mut functions = Vec::new();
    let mut init_func_idx = None;

    for (i, mir_func) in mir_module.functions.iter().enumerate() {
        let is_init = mir_func.name == "__init_module";
        if is_init {
            init_func_idx = Some(FuncIdx(i as u16));
        }

        let return_ty = ctx.lower_type(mir_func.return_type);
        for &param_ty in &mir_func.parameter_types {
            ctx.lower_type(param_ty);
        }
        for local_decl in &mir_func.locals {
            ctx.lower_type(local_decl.ty);
        }
        let param_count = mir_func.parameter_types.len() as u16;
        let local_count = (mir_func.locals.len() as u16).saturating_sub(param_count);

        let mut emitter =
            control_flow::FnEmitter::new(&mut ctx, mir_func, param_count, local_count);
        let instructions = emitter.emit();

        functions.push(ImageFunction {
            name: mir_func.name.clone(),
            param_count: param_count.try_into().unwrap(),
            local_count,
            temp_count: emitter.temp_count_max,
            return_ty,
            instructions,
        });
    }

    let export_symbols = collect_exports(module_graph);
    let mut exports = Vec::new();
    for mir_func in &mir_module.functions {
        if mir_func.name == "__init_module" {
            continue;
        }
        let sym = SymbolId::new(mir_func.id.raw());
        if export_symbols.contains(&sym) {
            let func_idx = ctx.function_map[&mir_func.id];
            exports.push(ExportSlot {
                symbol_name: mir_func.name.clone(),
                func_idx,
            });
        }
    }

    ModuleImage {
        name: module_graph.source_id().raw().to_string(),
        constants: ctx.constant_pool,
        functions,
        types: ctx.types,
        struct_layouts: ctx.struct_layouts,
        choice_layouts: ctx.choice_layouts,
        imports: Vec::new(),
        exports,
        init_func_idx,
    }
}

fn collect_exports(graph: &ModuleGraph) -> HashSet<SymbolId> {
    let mut exports = HashSet::new();
    let syntax = graph.syntax();
    let resolution = match graph.resolution() {
        Some(res) => res,
        None => return exports,
    };
    if let Some(root_node) = syntax.root().and_then(|root| syntax.node(root)) {
        for &item in root_node.children() {
            if let Some(node) = syntax.node(item)
                && node.kind() == SyntaxNodeKind::ExportItem
                && let Some(inner) = node.first_child()
                && let Some(inner_node) = syntax.node(inner)
            {
                let ident_node = if inner_node.kind() == SyntaxNodeKind::FunctionItem {
                    syntax.first_child_of_kind(inner, SyntaxNodeKind::Identifier)
                } else {
                    Some(inner)
                };
                if let Some(ident) = ident_node
                    && let Some(sym) = resolution.declaration_symbol(ident)
                {
                    exports.insert(sym);
                }
            }
        }
    }
    exports
}
