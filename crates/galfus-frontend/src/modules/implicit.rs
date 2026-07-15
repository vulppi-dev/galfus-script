use crate::{SyntaxLayer, SyntaxNodeKind};
use galfus_core::NodeId;

pub struct ImplicitDependencies {
    pub has_range: bool,
    pub has_match: bool,
}

pub fn collect_implicit_dependencies(syntax: &SyntaxLayer, root: NodeId) -> ImplicitDependencies {
    let mut deps = ImplicitDependencies {
        has_range: false,
        has_match: false,
    };

    collect_compiler_known_uses(syntax, root, &mut deps);
    deps
}

fn collect_compiler_known_uses(
    syntax: &SyntaxLayer,
    node_id: NodeId,
    deps: &mut ImplicitDependencies,
) {
    let Some(node) = syntax.node(node_id) else {
        return;
    };

    match node.kind() {
        SyntaxNodeKind::RangeExpression | SyntaxNodeKind::ForStatement => deps.has_range = true,
        SyntaxNodeKind::MatchExpression => deps.has_match = true,
        _ => {}
    }

    for child in node.children() {
        collect_compiler_known_uses(syntax, *child, deps);
    }
}
