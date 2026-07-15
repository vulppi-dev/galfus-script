use galfus_core::{FunctionId, NodeId, SymbolId, TypeId};
use galfus_frontend::{
    FunctionParameterType, FunctionType, PrimitiveType, SymbolKind, SyntaxNodeKind, TypeKind,
    TypeTable,
};
use galfus_ir::builder::WorkspaceContext;
use galfus_ir::mir::{
    BasicBlock, BlockId, Instruction, LocalDecl, LocalId, MirFunction, Operand, Terminator,
};
use std::collections::HashMap;

use crate::check::CheckedModule;

use super::resolve::resolve_import_target;

pub(super) struct MyWorkspaceContext<'a> {
    modules: &'a [CheckedModule],
    specialisations: HashMap<(usize, SymbolId, Vec<TypeId>), FunctionId>,
    pub(super) specialised_functions: Vec<Vec<MirFunction>>,
    pub(super) specialised_id_to_target: HashMap<FunctionId, (usize, FunctionId)>,
    next_specialised_id: u32,
}

impl<'a> MyWorkspaceContext<'a> {
    pub(super) fn new(modules: &'a [CheckedModule]) -> Self {
        Self {
            modules,
            specialisations: HashMap::new(),
            specialised_functions: vec![Vec::new(); modules.len()],
            specialised_id_to_target: HashMap::new(),
            next_specialised_id: 0x4000_0000,
        }
    }

    fn translate_symbol(
        &self,
        caller_mod_idx: usize,
        target_mod_idx: usize,
        sym: SymbolId,
    ) -> SymbolId {
        let caller_res = match self.modules[caller_mod_idx].graph().resolution() {
            Some(res) => res,
            None => return sym,
        };
        let caller_sym_data = match caller_res.symbol(sym) {
            Some(s) => s,
            None => return sym,
        };
        let sym_name = caller_sym_data.name();

        let target_res = match self.modules[target_mod_idx].graph().resolution() {
            Some(res) => res,
            None => return sym,
        };

        for target_sym in target_res.symbols() {
            if target_sym.name() == sym_name {
                return target_sym.id();
            }
        }

        for import in target_res.imports() {
            if import.local_name() == sym_name {
                return import.local_symbol();
            }
        }

        sym
    }

    fn translate_type(&self, caller_mod_idx: usize, target_mod_idx: usize, ty: TypeId) -> TypeId {
        let caller_module = &self.modules[caller_mod_idx];
        let caller_table = caller_module.type_result().unwrap().layer().table();

        let modules_ptr = self.modules.as_ptr() as usize as *mut CheckedModule;
        let target_module_mut = unsafe { &mut *modules_ptr.add(target_mod_idx) };
        let target_table = target_module_mut
            .type_result_mut()
            .unwrap()
            .layer_mut()
            .table_mut();

        self.translate_type_helper(
            caller_mod_idx,
            target_mod_idx,
            caller_table,
            target_table,
            ty,
        )
    }

    fn translate_type_helper(
        &self,
        caller_mod_idx: usize,
        target_mod_idx: usize,
        caller_table: &TypeTable,
        target_table: &mut TypeTable,
        ty: TypeId,
    ) -> TypeId {
        let kind = match caller_table.kind(ty) {
            Some(k) => k,
            None => return ty,
        };

        let translated_kind = match kind {
            TypeKind::Primitive(prim) => TypeKind::Primitive(*prim),
            TypeKind::Named { symbol } => {
                let target_symbol = self.translate_symbol(caller_mod_idx, target_mod_idx, *symbol);
                TypeKind::Named {
                    symbol: target_symbol,
                }
            }
            TypeKind::GenericParameter { symbol } => {
                let target_symbol = self.translate_symbol(caller_mod_idx, target_mod_idx, *symbol);
                TypeKind::GenericParameter {
                    symbol: target_symbol,
                }
            }
            TypeKind::Array { element } => {
                let target_element = self.translate_type_helper(
                    caller_mod_idx,
                    target_mod_idx,
                    caller_table,
                    target_table,
                    *element,
                );
                TypeKind::Array {
                    element: target_element,
                }
            }
            TypeKind::FixedArray { element, size } => {
                let target_element = self.translate_type_helper(
                    caller_mod_idx,
                    target_mod_idx,
                    caller_table,
                    target_table,
                    *element,
                );
                TypeKind::FixedArray {
                    element: target_element,
                    size: *size,
                }
            }
            TypeKind::Range { element } => {
                let target_element = self.translate_type_helper(
                    caller_mod_idx,
                    target_mod_idx,
                    caller_table,
                    target_table,
                    *element,
                );
                TypeKind::Range {
                    element: target_element,
                }
            }
            TypeKind::Tuple { elements } => {
                let target_elements = elements
                    .iter()
                    .map(|&e| {
                        self.translate_type_helper(
                            caller_mod_idx,
                            target_mod_idx,
                            caller_table,
                            target_table,
                            e,
                        )
                    })
                    .collect::<Vec<_>>();
                TypeKind::Tuple {
                    elements: target_elements,
                }
            }
            TypeKind::Union { members } => {
                let target_members = members
                    .iter()
                    .map(|&e| {
                        self.translate_type_helper(
                            caller_mod_idx,
                            target_mod_idx,
                            caller_table,
                            target_table,
                            e,
                        )
                    })
                    .collect::<Vec<_>>();
                TypeKind::Union {
                    members: target_members,
                }
            }
            TypeKind::Function(func) => {
                let target_return_type = self.translate_type_helper(
                    caller_mod_idx,
                    target_mod_idx,
                    caller_table,
                    target_table,
                    func.return_type(),
                );
                let target_parameters = func
                    .parameters()
                    .iter()
                    .map(|param| {
                        let target_ty = self.translate_type_helper(
                            caller_mod_idx,
                            target_mod_idx,
                            caller_table,
                            target_table,
                            param.ty(),
                        );
                        if param.is_rest() {
                            FunctionParameterType::rest(target_ty)
                        } else if param.has_default() {
                            FunctionParameterType::with_default(target_ty)
                        } else {
                            FunctionParameterType::new(target_ty)
                        }
                    })
                    .collect::<Vec<_>>();
                TypeKind::Function(FunctionType::new(target_parameters, target_return_type))
            }
            TypeKind::GenericInstance { base, arguments } => {
                let target_base = self.translate_type_helper(
                    caller_mod_idx,
                    target_mod_idx,
                    caller_table,
                    target_table,
                    *base,
                );
                let target_arguments = arguments
                    .iter()
                    .map(|&arg| {
                        self.translate_type_helper(
                            caller_mod_idx,
                            target_mod_idx,
                            caller_table,
                            target_table,
                            arg,
                        )
                    })
                    .collect::<Vec<_>>();
                TypeKind::GenericInstance {
                    base: target_base,
                    arguments: target_arguments,
                }
            }
            TypeKind::Path { root, segments } => {
                let target_root = self.translate_symbol(caller_mod_idx, target_mod_idx, *root);
                TypeKind::Path {
                    root: target_root,
                    segments: segments.clone(),
                }
            }
            TypeKind::Error => TypeKind::Error,
        };
        target_table.intern(translated_kind)
    }

    fn specialize_format_parse(
        &mut self,
        target_mod_idx: usize,
        target_symbol: SymbolId,
        concrete_types: &[TypeId],
        substitutions: HashMap<SymbolId, TypeId>,
    ) -> Option<FunctionId> {
        let target_module = &self.modules[target_mod_idx];

        if target_module.source().name() != "format"
            && target_module.path().to_string_lossy() != "format"
        {
            return None;
        }

        let resolution = target_module.graph().resolution()?;
        if resolution.symbol(target_symbol)?.name() != "parse" {
            return None;
        }

        let concrete_type = concrete_types.first().copied()?;
        let target_type_result = target_module.type_result()?;
        let parse_function_name = match target_type_result.layer().table().kind(concrete_type)? {
            TypeKind::Primitive(PrimitiveType::Int8) => "parseInt8",
            TypeKind::Primitive(PrimitiveType::Int16) => "parseInt16",
            TypeKind::Primitive(PrimitiveType::Int32) => "parseInt32",
            TypeKind::Primitive(PrimitiveType::Int64) => "parseInt64Raw",
            TypeKind::Primitive(PrimitiveType::Uint8) => "parseUint8",
            TypeKind::Primitive(PrimitiveType::Uint16) => "parseUint16",
            TypeKind::Primitive(PrimitiveType::Uint32) => "parseUint32",
            TypeKind::Primitive(PrimitiveType::Uint64) => "parseUint64Raw",
            TypeKind::Primitive(PrimitiveType::Float32) => "parseFloat32",
            TypeKind::Primitive(PrimitiveType::Float64) => "parseFloat64",
            TypeKind::Primitive(PrimitiveType::Bool) => "parseBoolRaw",
            TypeKind::Primitive(PrimitiveType::Null) => "parseNull",
            _ => return None,
        };

        let parse_function_id = resolution
            .symbols()
            .iter()
            .find(|symbol| {
                symbol.kind() == SymbolKind::Function && symbol.name() == parse_function_name
            })
            .map(|symbol| FunctionId::new(symbol.id().raw()))?;

        let key = (target_mod_idx, target_symbol, concrete_types.to_vec());
        if let Some(func_id) = self.specialisations.get(&key).copied() {
            return Some(func_id);
        }

        let specialized_id = FunctionId::new(self.next_specialised_id);
        self.next_specialised_id = self.next_specialised_id.saturating_sub(1);
        self.specialisations.insert(key, specialized_id);
        self.specialised_id_to_target
            .insert(specialized_id, (target_mod_idx, specialized_id));

        let type_res = target_module.type_result()?;
        let mut builder = galfus_ir::builder::MirBuilder::new(
            target_module.graph(),
            type_res,
            target_module.source().text(),
        );

        let function_item = builder.function_item_for_symbol(target_symbol)?;
        let mut function = builder.build_function_with_substitutions(
            function_item,
            Some(specialized_id),
            substitutions,
        )?;

        let result_id = function
            .locals
            .iter()
            .map(|local| local.id.raw())
            .max()
            .map(|id| LocalId::new(id + 1))
            .unwrap_or_else(|| LocalId::new(function.parameter_types.len() as u32));

        function.locals.push(LocalDecl {
            id: result_id,
            ty: function.return_type,
        });
        function.name = format!("{}#{}", function.name, specialized_id.raw());
        function.blocks = vec![
            BasicBlock {
                parameters: Vec::new(),
                id: BlockId::new(0),
                instructions: vec![Instruction::Call {
                    func: parse_function_id,
                    args: vec![Operand::Local(LocalId::new(0))],
                    destination: result_id,
                }],
                terminator: Terminator::Return(Some(Operand::Local(result_id))),
            },
        ];

        self.specialised_functions[target_mod_idx].push(function);

        Some(specialized_id)
    }
}

impl<'a> WorkspaceContext for MyWorkspaceContext<'a> {
    fn resolve_import(&self, node_id: NodeId) -> Option<(usize, SymbolId)> {
        let current_mod_idx = self
            .modules
            .iter()
            .position(|m| m.graph().syntax().node(node_id).is_some())?;

        let mut real_target = node_id;
        let module = &self.modules[current_mod_idx];
        let syntax = module.graph().syntax();
        while let Some(node) = syntax.node(real_target)
            && node.kind() == SyntaxNodeKind::GenericExpression
        {
            if let Some(inner) = node.first_child() {
                real_target = inner;
            } else {
                break;
            }
        }

        let func_id = FunctionId::new(0x8000_0000 | real_target.raw());
        let (target_mod_idx, target_func_id) =
            resolve_import_target(self.modules, current_mod_idx, func_id)?;
        let target_symbol = SymbolId::new(target_func_id.raw());
        Some((target_mod_idx, target_symbol))
    }

    fn get_generic_params(
        &self,
        target_mod_idx: usize,
        target_symbol: SymbolId,
    ) -> Option<Vec<SymbolId>> {
        let target_module = &self.modules[target_mod_idx];
        let type_res = target_module.type_result().unwrap();
        let builder = galfus_ir::builder::MirBuilder::new(
            target_module.graph(),
            type_res,
            target_module.source().text(),
        );
        let function_item = builder.function_item_for_symbol(target_symbol)?;
        Some(builder.generic_parameters_for_function_item(function_item))
    }

    fn specialize_function(
        &mut self,
        caller_node_id: NodeId,
        target_mod_idx: usize,
        target_symbol: SymbolId,
        concrete_types: Vec<TypeId>,
        substitutions: std::collections::HashMap<SymbolId, TypeId>,
    ) -> FunctionId {
        let caller_mod_idx = self
            .modules
            .iter()
            .position(|m| m.graph().syntax().node(caller_node_id).is_some())
            .unwrap_or(0);

        let concrete_types = concrete_types
            .iter()
            .map(|&ty| self.translate_type(caller_mod_idx, target_mod_idx, ty))
            .collect::<Vec<_>>();

        let substitutions = substitutions
            .into_iter()
            .map(|(sym, ty)| {
                let translated_ty = self.translate_type(caller_mod_idx, target_mod_idx, ty);
                (sym, translated_ty)
            })
            .collect::<HashMap<_, _>>();

        if let Some(specialized_id) = self.specialize_format_parse(
            target_mod_idx,
            target_symbol,
            &concrete_types,
            substitutions.clone(),
        ) {
            return specialized_id;
        }

        let key = (target_mod_idx, target_symbol, concrete_types.clone());
        if let Some(func_id) = self.specialisations.get(&key).copied() {
            return func_id;
        }

        let specialized_id = FunctionId::new(self.next_specialised_id);
        self.next_specialised_id = self.next_specialised_id.saturating_sub(1);
        self.specialisations.insert(key, specialized_id);
        self.specialised_id_to_target
            .insert(specialized_id, (target_mod_idx, specialized_id));

        let target_module = &self.modules[target_mod_idx];
        let type_res = target_module.type_result().unwrap();
        let mut builder = galfus_ir::builder::MirBuilder::new(
            target_module.graph(),
            type_res,
            target_module.source().text(),
        );
        builder = builder.with_workspace_ctx(self);

        if let Some(function_item) = builder.function_item_for_symbol(target_symbol) {
            if let Some(mut function) = builder.build_function_with_substitutions(
                function_item,
                Some(specialized_id),
                substitutions,
            ) {
                function.name = format!("{}#{}", function.name, specialized_id.raw());
                self.specialised_functions[target_mod_idx].push(function);
            }
        }

        specialized_id
    }

    fn specialize_builtin_function(
        &mut self,
        caller_node_id: NodeId,
        module_name: &str,
        function_name: &str,
        concrete_types: Vec<TypeId>,
    ) -> Option<FunctionId> {
        let target_mod_idx_opt = self
            .modules
            .iter()
            .position(|module| module.path().to_string_lossy() == module_name);
        let target_mod_idx = target_mod_idx_opt?;
        let resolution = self.modules[target_mod_idx].graph().resolution()?;
        let target_symbol = resolution
            .symbols()
            .iter()
            .find(|symbol| symbol.kind() == SymbolKind::Function && symbol.name() == function_name)
            .map(|symbol| symbol.id())?;
        let generic_params = self.get_generic_params(target_mod_idx, target_symbol)?;
        if generic_params.len() != concrete_types.len() {
            return None;
        }
        let substitutions = generic_params
            .into_iter()
            .zip(concrete_types.clone())
            .collect();
        Some(self.specialize_function(
            caller_node_id,
            target_mod_idx,
            target_symbol,
            concrete_types,
            substitutions,
        ))
    }

    fn function_return_type(&self, func_id: FunctionId) -> Option<TypeId> {
        self.specialised_functions
            .iter()
            .flat_map(|funcs| funcs.iter())
            .find(|f| f.id == func_id)
            .map(|f| f.return_type)
    }
}
