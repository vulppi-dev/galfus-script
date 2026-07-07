use anyhow::Result;
use galfus_core::{FunctionId, NodeId, SymbolId, TypeId};
use galfus_frontend::{
    FunctionParameterType, FunctionType, SymbolKind, SyntaxNodeKind, TypeKind, TypeTable,
};
use galfus_image::{
    ConstantPool, ImageFunction, ImageType, ModuleImage,
    instruction::{FuncIdx, Instruction, Reg, TypeIdx},
};
use galfus_ir::builder::WorkspaceContext;
use galfus_ir::mir::MirFunction;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::{CheckedModule, WorkspaceCheckResult, WorkspaceResolver, WorkspaceRootKind};

pub fn compile_workspace_to_gfb(
    check_result: &WorkspaceCheckResult,
    output_path: &Path,
) -> Result<()> {
    let module_image = compile_workspace_to_image(check_result)?;
    let gfb_bytes = galfus_image::gfb::serialize_to_gfb(&module_image)
        .map_err(|error| anyhow::anyhow!("Serialization error: {}", error))?;

    std::fs::write(output_path, gfb_bytes)?;

    Ok(())
}

struct MyWorkspaceContext<'a> {
    modules: &'a [CheckedModule],
    specialisations: HashMap<(usize, SymbolId, Vec<TypeId>), FunctionId>,
    specialised_functions: Vec<Vec<MirFunction>>,
    specialised_id_to_target: HashMap<FunctionId, (usize, FunctionId)>,
    next_specialised_id: u32,
}

impl<'a> MyWorkspaceContext<'a> {
    fn new(modules: &'a [CheckedModule]) -> Self {
        Self {
            modules,
            specialisations: HashMap::new(),
            specialised_functions: vec![Vec::new(); modules.len()],
            specialised_id_to_target: HashMap::new(),
            next_specialised_id: 0x7FFF_FFFF,
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

        // 1. Look in target's internal/declared symbols
        for target_sym in target_res.symbols() {
            if target_sym.name() == sym_name {
                return target_sym.id();
            }
        }

        // 2. Look in target's imports
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
}

pub fn compile_workspace_to_image(check_result: &WorkspaceCheckResult) -> Result<ModuleImage> {
    let modules = check_result.modules();
    let mut ws_ctx = MyWorkspaceContext::new(modules);

    let mut mir_modules = Vec::new();
    for module in modules {
        let type_res = module.type_result().ok_or_else(|| {
            anyhow::anyhow!(
                "Module is missing type checking result: {:?}",
                module.path()
            )
        })?;
        let mir =
            galfus_ir::builder::MirBuilder::new(module.graph(), type_res, module.source().text())
                .with_workspace_ctx(&mut ws_ctx)
                .build();
        mir_modules.push(mir);
    }

    for (i, mir_mod) in mir_modules.iter_mut().enumerate() {
        let mut specialized = std::mem::take(&mut ws_ctx.specialised_functions[i]);
        mir_mod.functions.append(&mut specialized);
    }

    // 1. Assign unique global function indices
    let mut next_global_func_idx = 0u16;
    let mut global_func_map = HashMap::new();
    for (mod_idx, mir_mod) in mir_modules.iter().enumerate() {
        for func in &mir_mod.functions {
            global_func_map.insert((mod_idx, func.id), FuncIdx(next_global_func_idx));
            next_global_func_idx += 1;
        }
    }

    // 2. Resolve imported function call targets and add them to global_func_map
    for (mod_idx, mir_mod) in mir_modules.iter().enumerate().take(modules.len()) {
        for func in &mir_mod.functions {
            let mut call_targets = Vec::new();
            collect_call_targets(&func.body, &mut call_targets);

            for func_id in call_targets {
                let resolved = if let Some(&(target_mod_idx, target_func_id)) =
                    ws_ctx.specialised_id_to_target.get(&func_id)
                {
                    Some((target_mod_idx, target_func_id))
                } else {
                    resolve_import_target(modules, mod_idx, func_id)
                };
                if let Some((target_mod_idx, target_func_id)) = resolved {
                    if let Some(&global_idx) =
                        global_func_map.get(&(target_mod_idx, target_func_id))
                    {
                        global_func_map.insert((mod_idx, func_id), global_idx);
                        continue;
                    }
                }

                if let Some(local_func_id) =
                    resolve_local_call_target(modules, mod_idx, mir_mod, func_id)
                    && let Some(&global_idx) = global_func_map.get(&(mod_idx, local_func_id))
                {
                    global_func_map.insert((mod_idx, func_id), global_idx);
                    continue;
                }

                if mir_mod
                    .functions
                    .iter()
                    .any(|local_func| local_func.id == func_id)
                {
                    continue;
                }

                return Err(anyhow::anyhow!(
                    "could not resolve function call target `{}` in module `{}`",
                    func_id.raw(),
                    modules[mod_idx].path().display()
                ));
            }
        }
    }

    // 3. Lower all modules under shared accumulators
    let mut types = Vec::new();
    let mut struct_layouts = Vec::new();
    let mut choice_layouts = Vec::new();
    let mut constant_pool = ConstantPool::default();
    let mut constants_map = HashMap::new();

    let mut functions = Vec::new();
    let mut init_funcs = HashMap::new();

    let mut next_global_idx = 0u16;
    let mut global_var_map = HashMap::new();

    for (mod_idx, mir_mod) in mir_modules.iter().enumerate() {
        let module = &modules[mod_idx];
        let type_res = module.type_result().unwrap();

        let mut ctx =
            galfus_ir::lower::LowerCtx::new(type_res, module.graph(), module.source().text());

        ctx.types = std::mem::take(&mut types);
        ctx.struct_layouts = std::mem::take(&mut struct_layouts);
        ctx.choice_layouts = std::mem::take(&mut choice_layouts);
        ctx.constant_pool = std::mem::take(&mut constant_pool);
        ctx.constants_map = std::mem::take(&mut constants_map);

        for (&(m_idx, func_id), &global_idx) in &global_func_map {
            if m_idx == mod_idx {
                ctx.function_map.insert(func_id, global_idx);
            }
        }

        for mir_func in &mir_mod.functions {
            ctx.function_names
                .insert(mir_func.id, mir_func.name.clone());
            ctx.function_return_types
                .insert(mir_func.id, mir_func.return_type);
        }

        for mir_func in &mir_mod.functions {
            let is_init = mir_func.name == "__init_module";
            let global_idx = global_func_map[&(mod_idx, mir_func.id)];

            if is_init {
                init_funcs.insert(mod_idx, global_idx);
            }

            ctx.active_substitutions = mir_func.type_substitutions.clone();

            let return_ty = ctx.lower_type(mir_func.return_type);
            for &param_ty in &mir_func.parameter_types {
                ctx.lower_type(param_ty);
            }
            for local_decl in &mir_func.locals {
                ctx.lower_type(local_decl.ty);
            }

            let param_count = mir_func.parameter_types.len() as u16;
            let local_count = image_local_count(mir_func, param_count);

            let mut emitter = galfus_ir::lower::control_flow::FnEmitter::new(
                &mut ctx,
                mir_func,
                param_count,
                local_count,
            );
            let mut instructions = emitter.emit();

            rewrite_global_indices(
                &mut instructions,
                modules,
                mod_idx,
                &mut global_var_map,
                &mut next_global_idx,
            )?;

            let img_func = ImageFunction {
                name: mir_func.name.clone(),
                param_count: param_count.try_into().unwrap(),
                local_count,
                temp_count: emitter.temp_count_max,
                return_ty,
                instructions,
            };

            functions.push(img_func);
        }

        types = std::mem::take(&mut ctx.types);
        struct_layouts = std::mem::take(&mut ctx.struct_layouts);
        choice_layouts = std::mem::take(&mut ctx.choice_layouts);
        constant_pool = std::mem::take(&mut ctx.constant_pool);
        constants_map = std::mem::take(&mut ctx.constants_map);
    }

    let entry_path = check_result
        .graph()
        .roots()
        .iter()
        .find(|r| matches!(r.kind(), WorkspaceRootKind::Entry))
        .map(|r| r.path().to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("No entrypoint defined in workspace config"))?;

    let entry_idx = modules
        .iter()
        .position(|m| m.path() == entry_path)
        .ok_or_else(|| anyhow::anyhow!("Entry module not found in workspace checked modules"))?;

    let entry_mir = &mir_modules[entry_idx];

    let ordered_init_funcs = order_workspace_init_funcs(modules, entry_idx, &init_funcs);

    let init_func_idx = if ordered_init_funcs.is_empty() {
        None
    } else {
        let mut init_instructions = Vec::new();

        for init_idx in ordered_init_funcs {
            init_instructions.push(Instruction::Call {
                dest: Reg(0),
                func: init_idx,
                args_start: Reg(0),
                arg_count: 0,
            });
        }
        init_instructions.push(Instruction::RetNull);

        let null_type_idx =
            if let Some(pos) = types.iter().position(|t| matches!(t, ImageType::Null)) {
                TypeIdx(pos as u16)
            } else {
                let idx = TypeIdx(types.len() as u16);
                types.push(ImageType::Null);
                idx
            };

        let synthetic_func_idx = FuncIdx(functions.len() as u16);
        functions.push(ImageFunction {
            name: "__init_workspace".to_string(),
            param_count: 0,
            local_count: 0,
            temp_count: 1,
            return_ty: null_type_idx,
            instructions: init_instructions,
        });
        Some(synthetic_func_idx)
    };

    let exports =
        collect_entry_exports(&modules[entry_idx], entry_mir, &global_func_map, entry_idx);

    let module_image = ModuleImage {
        name: check_result
            .graph()
            .roots()
            .first()
            .map(|r| r.path().to_string_lossy().into_owned())
            .unwrap_or_default(),
        constants: constant_pool,
        functions,
        types,
        struct_layouts,
        choice_layouts,
        imports: Vec::new(),
        exports,
        init_func_idx,
    };

    if let Err(errors) = galfus_image::validation::validate_module_image(&module_image) {
        return Err(anyhow::anyhow!(
            "ModuleImage validation failed: {:?}",
            errors
        ));
    }

    Ok(module_image)
}

fn collect_call_targets(body: &galfus_ir::mir::MirBody, targets: &mut Vec<FunctionId>) {
    match body {
        galfus_ir::mir::MirBody::BasicBlock(bb) => {
            if let galfus_ir::mir::Terminator::Call { func, .. } = &bb.terminator {
                targets.push(*func);
            }
        }
        galfus_ir::mir::MirBody::Block { statements, .. } => {
            for stmt in statements {
                collect_call_targets(stmt, targets);
            }
        }
        galfus_ir::mir::MirBody::If {
            then_branch,
            else_branch,
            ..
        } => {
            collect_call_targets(then_branch, targets);
            if let Some(else_b) = else_branch {
                collect_call_targets(else_b, targets);
            }
        }
        galfus_ir::mir::MirBody::Loop { body } => {
            collect_call_targets(body, targets);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitVisitState {
    Visiting,
    Visited,
}

fn order_workspace_init_funcs(
    modules: &[CheckedModule],
    entry_idx: usize,
    init_funcs: &HashMap<usize, FuncIdx>,
) -> Vec<FuncIdx> {
    let mut states = HashMap::new();
    let mut module_order = Vec::new();

    visit_init_dependencies(modules, entry_idx, &mut states, &mut module_order);

    module_order
        .into_iter()
        .filter_map(|module_idx| init_funcs.get(&module_idx).copied())
        .collect()
}

fn visit_init_dependencies(
    modules: &[CheckedModule],
    module_idx: usize,
    states: &mut HashMap<usize, InitVisitState>,
    module_order: &mut Vec<usize>,
) {
    match states.get(&module_idx).copied() {
        Some(InitVisitState::Visited) => return,
        Some(InitVisitState::Visiting) => return,
        None => {}
    }

    states.insert(module_idx, InitVisitState::Visiting);

    if let Some(resolution) = modules[module_idx].graph().resolution() {
        for import in resolution.imports() {
            if let Some(target_idx) = import_target_index(modules, module_idx, import.source()) {
                visit_init_dependencies(modules, target_idx, states, module_order);
            }
        }
    }

    states.insert(module_idx, InitVisitState::Visited);
    module_order.push(module_idx);
}

fn resolve_import_target(
    modules: &[CheckedModule],
    mod_idx: usize,
    func_id: FunctionId,
) -> Option<(usize, FunctionId)> {
    let module = &modules[mod_idx];
    let resolution = module.graph().resolution()?;
    let symbol_id = SymbolId::new(func_id.raw());
    let node_id = path_call_target_node(func_id).unwrap_or_else(|| NodeId::new(func_id.raw()));

    if let Some(syntax_node) = module.graph().syntax().node(node_id) {
        if syntax_node.kind() == SyntaxNodeKind::PathExpression
            && let Some(root_node) = syntax_node.first_child()
        {
            let root_sym = resolution.reference_symbol(root_node);
            if let Some(root_symbol) = root_sym {
                let import_found = resolution
                    .imports()
                    .iter()
                    .find(|imp| imp.local_symbol() == root_symbol);
                if let Some(import) = import_found
                    && let Some(member_node) = syntax_node.child(1)
                {
                    let syntax = module.graph().syntax();
                    let member_node_data = syntax.node(member_node)?;
                    let member_span = member_node_data.span();
                    let member_name = module.source().slice(member_span)?;
                    let target_idx = import_target_index(modules, mod_idx, import.source());
                    if let Some(target_idx) = target_idx {
                        let target_mod = &modules[target_idx];
                        let target_resolution = target_mod.graph().resolution()?;
                        for export in target_resolution.exports() {
                            if export.name() == member_name {
                                return Some((target_idx, FunctionId::new(export.symbol().raw())));
                            }
                        }
                    }
                }
            }
        }
    }

    // Check if symbol_id is a Named Import after PathExpression handling. NodeId
    // based call targets may numerically collide with imported SymbolIds.
    if let Some(import) = resolution
        .imports()
        .iter()
        .find(|imp| imp.local_symbol() == symbol_id)
        && let Some(imported_name) = import.imported_name()
    {
        let target_idx = import_target_index(modules, mod_idx, import.source())?;

        let target_mod = &modules[target_idx];
        let target_resolution = target_mod.graph().resolution()?;
        for export in target_resolution.exports() {
            if export.name() == imported_name {
                return Some((target_idx, FunctionId::new(export.symbol().raw())));
            }
        }
    }

    if let Some(syntax_node) = module.graph().syntax().node(node_id)
        && syntax_node.kind() == SyntaxNodeKind::PathExpression
        && let Some(member_node) = syntax_node.child(1)
        && let Some(member_node_data) = module.graph().syntax().node(member_node)
        && let Some(member_name) = module.source().slice(member_node_data.span())
        && let Some(receiver) = syntax_node.child(0)
        && let Some(source) = import_source_for_expression(module, receiver)
        && let Some(target_idx) = import_target_index(modules, mod_idx, source)
        && let Some(target_resolution) = modules[target_idx].graph().resolution()
    {
        let mut candidates = target_resolution.exports().iter().filter_map(|export| {
            let matches_member = export.name() == member_name
                || export.name().ends_with(&format!("::{member_name}"));
            (export.kind() == SymbolKind::Function && matches_member)
                .then_some((target_idx, FunctionId::new(export.symbol().raw())))
        });
        let first = candidates.next();
        if first.is_some() && candidates.next().is_none() {
            return first;
        }
    }

    let mut candidates = modules[mod_idx]
        .graph()
        .resolution()?
        .imports()
        .iter()
        .filter_map(|import| {
            let target_idx = import_target_index(modules, mod_idx, import.source())?;
            let target_resolution = modules[target_idx].graph().resolution()?;
            target_resolution
                .exports()
                .iter()
                .find(|export| {
                    export.kind() == SymbolKind::Function && export.symbol().raw() == func_id.raw()
                })
                .map(|export| (target_idx, FunctionId::new(export.symbol().raw())))
        });
    let first = candidates.next();
    if first.is_some() && candidates.next().is_none() {
        return first;
    }

    None
}

fn resolve_local_call_target(
    modules: &[CheckedModule],
    mod_idx: usize,
    mir_mod: &galfus_ir::mir::MirModule,
    func_id: FunctionId,
) -> Option<FunctionId> {
    let module = &modules[mod_idx];
    let node_id = path_call_target_node(func_id)?;
    let node = module.graph().syntax().node(node_id)?;
    if node.kind() != SyntaxNodeKind::PathExpression {
        return None;
    }
    if let Some(symbol) = module.graph().resolution()?.path_reference_symbol(node_id) {
        return Some(FunctionId::new(symbol.raw()));
    }

    let member_node = node.child(1)?;
    let member_node_data = module.graph().syntax().node(member_node)?;
    let member_name = module.source().slice(member_node_data.span())?;
    let mut candidates = mir_mod.functions.iter().filter_map(|function| {
        let matches_member =
            function.name == member_name || function.name.ends_with(&format!("::{member_name}"));
        matches_member.then_some(function.id)
    });
    let first = candidates.next();
    if first.is_some() && candidates.next().is_none() {
        return first;
    }

    None
}

const PATH_CALL_TARGET_TAG: u32 = 0x8000_0000;

fn path_call_target_node(func_id: FunctionId) -> Option<NodeId> {
    let raw = func_id.raw();
    (raw & PATH_CALL_TARGET_TAG != 0).then(|| NodeId::new(raw & !PATH_CALL_TARGET_TAG))
}

fn canonical_global_idx(
    modules: &[CheckedModule],
    mod_idx: usize,
    local_pos: u16,
    global_var_map: &mut HashMap<(usize, String), galfus_image::instruction::GlobalIdx>,
    next_global_idx: &mut u16,
) -> Result<galfus_image::instruction::GlobalIdx> {
    let module = modules
        .get(mod_idx)
        .ok_or_else(|| anyhow::anyhow!("invalid module index `{mod_idx}` during global rewrite"))?;

    let resolution = module.graph().resolution().ok_or_else(|| {
        anyhow::anyhow!(
            "missing resolver output for module `{}` during global rewrite",
            module.path().display()
        )
    })?;

    let symbols = resolution.symbols();
    let symbol = symbols.get(local_pos as usize).ok_or_else(|| {
        anyhow::anyhow!(
            "missing local/global symbol at position `{local_pos}` in module `{}`",
            module.path().display()
        )
    })?;

    if let Some(import) = resolution
        .imports()
        .iter()
        .find(|import| import.local_symbol() == symbol.id())
    {
        let imported_name = import.imported_name().ok_or_else(|| {
            anyhow::anyhow!(
                "module import `{}` in `{}` cannot be used as a global value directly",
                import.source(),
                module.path().display()
            )
        })?;

        let target_idx =
            import_target_index(modules, mod_idx, import.source()).ok_or_else(|| {
                anyhow::anyhow!(
                    "could not resolve import `{}` from module `{}` while rewriting global `{}`",
                    import.source(),
                    module.path().display(),
                    imported_name
                )
            })?;

        let key = (target_idx, imported_name.to_string());
        let idx = *global_var_map.entry(key).or_insert_with(|| {
            let idx = galfus_image::instruction::GlobalIdx(*next_global_idx);
            *next_global_idx += 1;
            idx
        });

        return Ok(idx);
    }

    let key = (mod_idx, symbol.name().to_string());
    let idx = *global_var_map.entry(key).or_insert_with(|| {
        let idx = galfus_image::instruction::GlobalIdx(*next_global_idx);
        *next_global_idx += 1;
        idx
    });

    Ok(idx)
}

fn import_target_index(modules: &[CheckedModule], mod_idx: usize, source: &str) -> Option<usize> {
    let resolver = WorkspaceResolver::default();
    let target = resolver
        .resolve_import(modules[mod_idx].path(), source)
        .ok()?
        .path();
    modules.iter().position(|module| module.path() == target)
}

fn import_source_for_expression(module: &CheckedModule, expr: NodeId) -> Option<&str> {
    let syntax = module.graph().syntax();
    let resolution = module.graph().resolution()?;
    let node = syntax.node(expr)?;

    match node.kind() {
        SyntaxNodeKind::CallExpression | SyntaxNodeKind::GenericExpression => {
            let target = node.child(0)?;
            import_source_for_expression(module, target)
        }
        SyntaxNodeKind::PathExpression => {
            let root = node.child(0)?;
            import_source_for_expression(module, root)
        }
        SyntaxNodeKind::NameExpression | SyntaxNodeKind::Identifier => {
            let symbol = resolution.reference_symbol(expr).or_else(|| {
                let ident = syntax.first_child_of_kind(expr, SyntaxNodeKind::Identifier)?;
                resolution.reference_symbol(ident)
            })?;
            resolution
                .imports()
                .iter()
                .find(|import| import.local_symbol() == symbol)
                .map(|import| import.source())
        }
        _ => None,
    }
}

fn rewrite_global_indices(
    instructions: &mut [galfus_image::instruction::Instruction],
    modules: &[CheckedModule],
    mod_idx: usize,
    global_var_map: &mut HashMap<(usize, String), galfus_image::instruction::GlobalIdx>,
    next_global_idx: &mut u16,
) -> Result<()> {
    use galfus_image::instruction::Instruction;

    for instr in instructions {
        match instr {
            Instruction::LoadGlobal {
                dest: _,
                global_idx,
            } => {
                *global_idx = canonical_global_idx(
                    modules,
                    mod_idx,
                    global_idx.raw(),
                    global_var_map,
                    next_global_idx,
                )?;
            }
            Instruction::StoreGlobal { global_idx, src: _ } => {
                *global_idx = canonical_global_idx(
                    modules,
                    mod_idx,
                    global_idx.raw(),
                    global_var_map,
                    next_global_idx,
                )?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn collect_entry_exports(
    entry_module: &CheckedModule,
    entry_mir: &galfus_ir::mir::MirModule,
    global_func_map: &HashMap<(usize, FunctionId), galfus_image::instruction::FuncIdx>,
    entry_idx: usize,
) -> Vec<galfus_image::ExportSlot> {
    let mut exports = Vec::new();
    let resolution = match entry_module.graph().resolution() {
        Some(res) => res,
        None => return exports,
    };
    let mut export_symbols = HashSet::new();
    for export in resolution.exports() {
        export_symbols.insert(export.symbol());
    }
    for func in &entry_mir.functions {
        if func.name == "__init_module" {
            continue;
        }
        let sym = SymbolId::new(func.id.raw());
        if export_symbols.contains(&sym)
            && let Some(&func_idx) = global_func_map.get(&(entry_idx, func.id))
        {
            exports.push(galfus_image::ExportSlot {
                symbol_name: func.name.clone(),
                func_idx,
            });
        }
    }
    exports
}

fn image_local_count(mir_func: &galfus_ir::mir::MirFunction, param_count: u16) -> u16 {
    let max_local_id = mir_func
        .locals
        .iter()
        .map(|local| local.id.raw() as u16)
        .max()
        .map(|max_id| max_id + 1)
        .unwrap_or(param_count);

    max_local_id.saturating_sub(param_count)
}
