use super::*;
use galfus_core::{SourceId, Span};

#[test]
fn syntax_layer_starts_empty() {
    let syntax = SyntaxLayer::new();

    assert!(syntax.root().is_none());
    assert!(syntax.tokens().is_empty());
    assert!(syntax.nodes().is_empty());
    assert!(syntax.is_empty());
    assert_eq!(syntax.len(), 0);
}

#[test]
fn syntax_layer_adds_node() {
    let source_id = SourceId::new(0);
    let span = Span::new(source_id, 0, 4);

    let mut syntax = SyntaxLayer::new();

    let id = syntax.add_node(SyntaxNodeKind::Identifier, span, Vec::new());

    assert_eq!(id, NodeId::new(0));
    assert_eq!(syntax.len(), 1);

    let node = syntax.node(id).unwrap();

    assert_eq!(node.kind(), SyntaxNodeKind::Identifier);
    assert_eq!(node.span(), span);
    assert!(node.children().is_empty());
}

#[test]
fn syntax_layer_stores_root() {
    let source_id = SourceId::new(0);
    let span = Span::new(source_id, 0, 0);

    let mut syntax = SyntaxLayer::new();

    let root = syntax.add_node(SyntaxNodeKind::SourceFile, span, Vec::new());

    syntax.set_root(root);

    assert_eq!(syntax.root(), Some(root));
}

#[test]
fn module_graph_has_syntax_layer() {
    let source_id = SourceId::new(0);
    let graph = ModuleGraph::new(source_id);

    assert_eq!(graph.source_id(), source_id);
    assert_eq!(graph.phase(), GraphPhase::Parsed);
    assert!(graph.syntax().is_empty());
    assert!(!graph.has_errors());
}
