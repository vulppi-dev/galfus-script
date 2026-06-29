use anyhow::Result;
use galfus_core::SymbolId;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::{CheckedModule, WorkspaceCheckResult, WorkspaceRootKind, normalize_existing_path};

pub fn compile_workspace_to_gfb(
    check_result: &WorkspaceCheckResult,
    output_path: &Path,
) -> Result<()> {
    use galfus_image::{
        ConstantPool, ImageFunction, ImageType, ModuleImage,
        instruction::{FuncIdx, Instruction, Reg, TypeIdx},
    };
    use std::fs;

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
    for mod_idx in 0..modules.len() {
        let mir_mod = &mir_modules[mod_idx];
        for func in &mir_mod.functions {
            let mut call_targets = Vec::new();
            collect_call_targets(&func.body, &mut call_targets);
            for func_id in call_targets {
                if !mir_mod.functions.iter().any(|f| f.id == func_id)
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
    let mut type_map = HashMap::new();
    let mut struct_map = HashMap::new();
    let mut choice_map = HashMap::new();
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
        ctx.type_map = std::mem::take(&mut type_map);
        ctx.struct_map = std::mem::take(&mut struct_map);
        ctx.choice_map = std::mem::take(&mut choice_map);
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
        type_map = std::mem::take(&mut ctx.type_map);
        struct_map = std::mem::take(&mut ctx.struct_map);
        choice_map = std::mem::take(&mut ctx.choice_map);
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

    let gfb_bytes = galfus_image::gfb::serialize_to_gfb(&module_image)
        .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;

    fs::write(output_path, gfb_bytes)?;

    Ok(())
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
    use galfus_frontend::SyntaxNodeKind;

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
        let target_path = if import.source() == "std/io" {
            PathBuf::from(import.source())
        } else {
            resolve_relative_import(module.path(), import.source())
        };
        let target_path = normalize_existing_path(target_path.as_path()).ok()?;
        let target_idx = modules.iter().position(|m| m.path() == target_path)?;

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
        let target_path = if import.source() == "std/io" {
            PathBuf::from(import.source())
        } else {
            resolve_relative_import(module.path(), import.source())
        };
        let target_path = normalize_existing_path(target_path.as_path()).ok()?;
        let target_idx = modules.iter().position(|m| m.path() == target_path)?;

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

    None
}

fn resolve_relative_import(base_module: &Path, source: &str) -> PathBuf {
    let base_dir = base_module.parent().unwrap_or_else(|| Path::new(""));
    let mut path = base_dir.join(source);

    if path.extension().is_none() {
        path.set_extension("gfs");
    }

    path
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
    {
        let target_path = if import.source() == "std/io" {
            PathBuf::from(import.source())
        } else {
            resolve_relative_import(module.path(), import.source())
        };
        if let Ok(target_path) = normalize_existing_path(target_path.as_path())
            && let Some(target_idx) = modules.iter().position(|m| m.path() == target_path)
        {
            let key = (target_idx, imported_name.to_string());
            return *global_var_map.entry(key).or_insert_with(|| {
                let idx = galfus_image::instruction::GlobalIdx(*next_global_idx);
                *next_global_idx += 1;
                idx
            });
        }
    }

    // Otherwise, s is local to mod_idx
    let key = (mod_idx, s.name().to_string());
    *global_var_map.entry(key).or_insert_with(|| {
        let idx = galfus_image::instruction::GlobalIdx(*next_global_idx);
        *next_global_idx += 1;
        idx
    })
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
