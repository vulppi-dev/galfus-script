use anyhow::Result;
use galfus_core::SymbolId;
use galfus_frontend::SyntaxNodeKind;
use galfus_image::{
    ConstantPool, ImageFunction, ImageType, ModuleImage,
    instruction::{FuncIdx, Instruction, Reg, TypeIdx},
};
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

pub fn compile_workspace_to_image(check_result: &WorkspaceCheckResult) -> Result<ModuleImage> {
    let modules = check_result.modules();
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
                .build();
        mir_modules.push(mir);
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
                if !mir_mod.functions.iter().any(|f| f.id == func_id)
                    && let Some(local_func_id) =
                        resolve_local_call_target(modules, mod_idx, mir_mod, func_id)
                    && let Some(&global_idx) = global_func_map.get(&(mod_idx, local_func_id))
                {
                    global_func_map.insert((mod_idx, func_id), global_idx);
                } else if !mir_mod.functions.iter().any(|f| f.id == func_id)
                    && let Some((target_mod_idx, target_func_id)) =
                        resolve_import_target(modules, mod_idx, func_id)
                    && let Some(&global_idx) =
                        global_func_map.get(&(target_mod_idx, target_func_id))
                {
                    global_func_map.insert((mod_idx, func_id), global_idx);
                }
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
    let mut init_funcs = Vec::new();

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
        }

        for mir_func in &mir_mod.functions {
            let is_init = mir_func.name == "__init_module";
            let global_idx = global_func_map[&(mod_idx, mir_func.id)];

            if is_init {
                init_funcs.push(global_idx);
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
            );

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

    let init_func_idx = if init_funcs.is_empty() {
        None
    } else {
        let mut init_instructions = Vec::new();
        for &init_idx in &init_funcs {
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

fn collect_call_targets(
    body: &galfus_ir::mir::MirBody,
    targets: &mut Vec<galfus_core::FunctionId>,
) {
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

fn resolve_import_target(
    modules: &[CheckedModule],
    mod_idx: usize,
    func_id: galfus_core::FunctionId,
) -> Option<(usize, galfus_core::FunctionId)> {
    use galfus_core::{NodeId, SymbolId};
    use galfus_frontend::SymbolKind;

    let module = &modules[mod_idx];
    let resolution = module.graph().resolution()?;
    let symbol_id = SymbolId::new(func_id.raw());

    // 1. Check if symbol_id is a Named Import
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
                return Some((
                    target_idx,
                    galfus_core::FunctionId::new(export.symbol().raw()),
                ));
            }
        }
    }

    // 2. Check if func_id is a Namespace Import call target (e.g. NodeId of a PathExpression)
    let node_id = NodeId::new(func_id.raw());
    if let Some(syntax_node) = module.graph().syntax().node(node_id)
        && syntax_node.kind() == SyntaxNodeKind::PathExpression
        && let Some(root_node) = syntax_node.first_child()
        && let Some(root_symbol) = resolution.reference_symbol(root_node)
        && let Some(import) = resolution
            .imports()
            .iter()
            .find(|imp| imp.local_symbol() == root_symbol)
        && let Some(member_node) = syntax_node.child(1)
    {
        let syntax = module.graph().syntax();
        let member_node_data = syntax.node(member_node)?;
        let member_span = member_node_data.span();
        let member_name = module.source().slice(member_span)?;
        let target_idx = import_target_index(modules, mod_idx, import.source())?;

        let target_mod = &modules[target_idx];
        let target_resolution = target_mod.graph().resolution()?;
        for export in target_resolution.exports() {
            if export.name() == member_name {
                return Some((
                    target_idx,
                    galfus_core::FunctionId::new(export.symbol().raw()),
                ));
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
            (export.kind() == SymbolKind::Function && matches_member).then_some((
                target_idx,
                galfus_core::FunctionId::new(export.symbol().raw()),
            ))
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
                .map(|export| {
                    (
                        target_idx,
                        galfus_core::FunctionId::new(export.symbol().raw()),
                    )
                })
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
    func_id: galfus_core::FunctionId,
) -> Option<galfus_core::FunctionId> {
    let module = &modules[mod_idx];
    let node_id = galfus_core::NodeId::new(func_id.raw());
    let node = module.graph().syntax().node(node_id)?;
    if node.kind() != SyntaxNodeKind::PathExpression {
        return None;
    }
    if let Some(symbol) = module.graph().resolution()?.path_reference_symbol(node_id) {
        return Some(galfus_core::FunctionId::new(symbol.raw()));
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

fn canonical_global_idx(
    modules: &[CheckedModule],
    mod_idx: usize,
    local_pos: u16,
    global_var_map: &mut HashMap<(usize, String), galfus_image::instruction::GlobalIdx>,
    next_global_idx: &mut u16,
) -> galfus_image::instruction::GlobalIdx {
    let module = &modules[mod_idx];
    let resolution = match module.graph().resolution() {
        Some(res) => res,
        None => return galfus_image::instruction::GlobalIdx(0),
    };
    let symbols = resolution.symbols();
    let s = match symbols.get(local_pos as usize) {
        Some(sym) => sym,
        None => return galfus_image::instruction::GlobalIdx(0),
    };

    // If s is an import, resolve it
    if let Some(import) = resolution
        .imports()
        .iter()
        .find(|imp| imp.local_symbol() == s.id())
        && let Some(imported_name) = import.imported_name()
        && let Some(target_idx) = import_target_index(modules, mod_idx, import.source())
    {
        let key = (target_idx, imported_name.to_string());
        return *global_var_map.entry(key).or_insert_with(|| {
            let idx = galfus_image::instruction::GlobalIdx(*next_global_idx);
            *next_global_idx += 1;
            idx
        });
    }

    // Otherwise, s is local to mod_idx
    let key = (mod_idx, s.name().to_string());
    *global_var_map.entry(key).or_insert_with(|| {
        let idx = galfus_image::instruction::GlobalIdx(*next_global_idx);
        *next_global_idx += 1;
        idx
    })
}

fn import_target_index(modules: &[CheckedModule], mod_idx: usize, source: &str) -> Option<usize> {
    let resolver = WorkspaceResolver::default();
    let target = resolver
        .resolve_import(modules[mod_idx].path(), source)
        .ok()?
        .path();
    modules.iter().position(|module| module.path() == target)
}

fn import_source_for_expression(module: &CheckedModule, expr: galfus_core::NodeId) -> Option<&str> {
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
) {
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
                );
            }
            Instruction::StoreGlobal { global_idx, src: _ } => {
                *global_idx = canonical_global_idx(
                    modules,
                    mod_idx,
                    global_idx.raw(),
                    global_var_map,
                    next_global_idx,
                );
            }
            _ => {}
        }
    }
}

fn collect_entry_exports(
    entry_module: &CheckedModule,
    entry_mir: &galfus_ir::mir::MirModule,
    global_func_map: &HashMap<(usize, galfus_core::FunctionId), galfus_image::instruction::FuncIdx>,
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
