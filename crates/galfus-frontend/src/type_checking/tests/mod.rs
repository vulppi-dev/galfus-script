mod access;
mod assignments;
mod calls;
mod declarations;
mod initializers;
mod literals;
mod operators;
mod returns;
mod structs;
mod variants;

use super::*;

use galfus_core::{DiagnosticCodeKind, NodeId, SourceId, SymbolId};

use crate::{ArraySize, SymbolKind, SyntaxNodeKind, TypeDiagnosticCode, TypeKind, parse, resolve};

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "test.gfs".to_string(), text.to_string())
}

fn check_source(text: &str) -> (SourceFile, ModuleGraph, TypeCheckResult) {
    let source = source(text);

    let parse_result = parse(&source);
    assert!(
        !parse_result.has_errors(),
        "{:?}",
        parse_result.diagnostics()
    );

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(
        !resolve_result.has_errors(),
        "{:?}",
        resolve_result.diagnostics()
    );

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(!result.has_errors(), "{:?}", result.diagnostics());

    (source, graph, result)
}

fn symbol_by_name_and_kind(graph: &ModuleGraph, name: &str, kind: SymbolKind) -> SymbolId {
    let resolution = graph.resolution().unwrap();

    resolution
        .symbols()
        .iter()
        .find(|symbol| symbol.name() == name && symbol.kind() == kind)
        .map(|symbol| symbol.id())
        .unwrap_or_else(|| panic!("missing symbol `{name}` of kind {kind:?}"))
}

fn find_node_by_kind_and_text(
    source: &SourceFile,
    graph: &ModuleGraph,
    kind: SyntaxNodeKind,
    text: &str,
) -> Option<NodeId> {
    let root = graph.syntax().root()?;
    find_node_by_kind_and_text_from(source, graph, root, kind, text)
}

fn find_node_by_kind_and_text_from(
    source: &SourceFile,
    graph: &ModuleGraph,
    node: NodeId,
    kind: SyntaxNodeKind,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = graph.syntax().node(node)?;

    if syntax_node.kind() == kind && source.slice(syntax_node.span()) == Some(text) {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_node_by_kind_and_text_from(source, graph, *child, kind, text) {
            return Some(found);
        }
    }

    None
}

fn find_node_by_kind(graph: &ModuleGraph, kind: SyntaxNodeKind) -> Option<NodeId> {
    let root = graph.syntax().root()?;
    find_node_by_kind_from(graph, root, kind)
}

fn find_node_by_kind_from(
    graph: &ModuleGraph,
    node: NodeId,
    kind: SyntaxNodeKind,
) -> Option<NodeId> {
    let syntax_node = graph.syntax().node(node)?;

    if syntax_node.kind() == kind {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_node_by_kind_from(graph, *child, kind) {
            return Some(found);
        }
    }

    None
}
