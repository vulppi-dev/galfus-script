use galfus_core::{FunctionId, NodeId, SymbolId};
use galfus_frontend::{SymbolKind, SyntaxNodeKind};

use crate::{WorkspaceResolver, check::CheckedModule};

pub(super) fn collect_call_targets(
    blocks: &[galfus_ir::mir::BasicBlock],
    targets: &mut Vec<FunctionId>,
) {
    for bb in blocks {
        for inst in &bb.instructions {
            if let galfus_ir::mir::Instruction::Call { func, .. } = inst {
                targets.push(*func);
            }
        }
    }
}

pub(super) fn resolve_import_target(
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

    let import_symbol = module
        .graph()
        .syntax()
        .node(node_id)
        .and_then(|_| resolution.reference_symbol(node_id))
        .unwrap_or(symbol_id);

    if let Some(import) = resolution
        .imports()
        .iter()
        .find(|imp| imp.local_symbol() == import_symbol)
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

pub(super) fn resolve_local_call_target(
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

pub(super) fn import_target_index(
    modules: &[CheckedModule],
    mod_idx: usize,
    source: &str,
) -> Option<usize> {
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
