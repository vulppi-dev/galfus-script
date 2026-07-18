use galfus_image::instruction::FuncIdx;
use std::collections::HashMap;

use crate::input::CompiledModule;

use super::resolve::import_target_index;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitVisitState {
    Visiting,
    Visited,
}

pub(super) fn order_workspace_init_funcs(
    modules: &[CompiledModule],
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
    modules: &[CompiledModule],
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
