use anyhow::Result;
use galfus_image::{
    ConstantPool, ImageFunction, ImageType, ModuleImage,
    instruction::{FuncIdx, Instruction, Reg, TypeIdx},
};
use std::collections::HashMap;

use crate::workspace::{WorkspaceCheckResult, WorkspaceRootKind};

mod context;
mod exports;
mod globals;
mod init;
mod resolve;

use context::MyWorkspaceContext;
use exports::collect_entry_exports;
use globals::{image_local_count, rewrite_global_indices};
use init::order_workspace_init_funcs;
use resolve::{collect_call_targets, resolve_import_target, resolve_local_call_target};

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

    let mut next_global_func_idx = 0u16;
    let mut global_func_map = HashMap::new();
    for (mod_idx, mir_mod) in mir_modules.iter().enumerate() {
        for func in &mir_mod.functions {
            global_func_map.insert((mod_idx, func.id), FuncIdx(next_global_func_idx));
            next_global_func_idx += 1;
        }
    }

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
