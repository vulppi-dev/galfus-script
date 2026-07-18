use anyhow::Result;
use galfus_image::instruction::{GlobalIdx, Instruction};
use std::collections::HashMap;

use crate::input::CompiledModule;

use super::resolve::import_target_index;

fn canonical_global_idx(
    modules: &[CompiledModule],
    mod_idx: usize,
    local_pos: u16,
    global_var_map: &mut HashMap<(usize, String), GlobalIdx>,
    next_global_idx: &mut u16,
) -> Result<GlobalIdx> {
    let module = modules
        .get(mod_idx)
        .ok_or_else(|| anyhow::anyhow!("invalid module index `{mod_idx}` during global rewrite"))?;

    let resolution = module.graph().resolution().ok_or_else(|| {
        anyhow::anyhow!(
            "missing resolver output for module `{}` during global rewrite",
            module.path().as_str()
        )
    })?;

    let symbols = resolution.symbols();
    let symbol = symbols.get(local_pos as usize).ok_or_else(|| {
        anyhow::anyhow!(
            "missing local/global symbol at position `{local_pos}` in module `{}`",
            module.path().as_str()
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
                module.path().as_str()
            )
        })?;

        let target_idx =
            import_target_index(modules, mod_idx, import.source()).ok_or_else(|| {
                anyhow::anyhow!(
                    "could not resolve import `{}` from module `{}` while rewriting global `{}`",
                    import.source(),
                    module.path().as_str(),
                    imported_name
                )
            })?;

        let key = (target_idx, imported_name.to_string());
        let idx = *global_var_map.entry(key).or_insert_with(|| {
            let idx = GlobalIdx(*next_global_idx);
            *next_global_idx += 1;
            idx
        });

        return Ok(idx);
    }

    let key = (mod_idx, symbol.name().to_string());
    let idx = *global_var_map.entry(key).or_insert_with(|| {
        let idx = GlobalIdx(*next_global_idx);
        *next_global_idx += 1;
        idx
    });

    Ok(idx)
}

pub(super) fn rewrite_global_indices(
    instructions: &mut [Instruction],
    modules: &[CompiledModule],
    mod_idx: usize,
    global_var_map: &mut HashMap<(usize, String), GlobalIdx>,
    next_global_idx: &mut u16,
) -> Result<()> {
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

pub(super) fn image_local_count(mir_func: &galfus_ir::mir::MirFunction, param_count: u16) -> u16 {
    let max_local_id = mir_func
        .locals
        .iter()
        .map(|local| local.id.raw() as u16)
        .max()
        .map(|max_id| max_id + 1)
        .unwrap_or(param_count);

    max_local_id.saturating_sub(param_count)
}
