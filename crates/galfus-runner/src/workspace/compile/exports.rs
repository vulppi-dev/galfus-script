use galfus_core::{FunctionId, SymbolId};
use galfus_image::instruction::FuncIdx;
use std::collections::{HashMap, HashSet};

use crate::check::CheckedModule;

pub(super) fn collect_entry_exports(
    entry_module: &CheckedModule,
    entry_mir: &galfus_ir::mir::MirModule,
    global_func_map: &HashMap<(usize, FunctionId), FuncIdx>,
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
