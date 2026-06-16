use super::*;
use crate::SyntaxLayer;
use galfus_core::NodeId;

fn find_first_of_kind(syntax: &SyntaxLayer, node: NodeId, kind: SyntaxNodeKind) -> Option<NodeId> {
    let syntax_node = syntax.node(node)?;

    if syntax_node.kind() == kind {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_first_of_kind(syntax, *child, kind) {
            return Some(found);
        }
    }

    None
}

fn find_name_expression_by_text(
    syntax: &SyntaxLayer,
    source: &SourceFile,
    node: NodeId,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = syntax.node(node)?;

    if syntax_node.kind() == SyntaxNodeKind::NameExpression {
        if let Some(identifier) = syntax.first_child_of_kind(node, SyntaxNodeKind::Identifier) {
            if source.slice(syntax.node(identifier)?.span()) == Some(text) {
                return Some(node);
            }
        }
    }

    for child in syntax_node.children() {
        if let Some(found) = find_name_expression_by_text(syntax, source, *child, text) {
            return Some(found);
        }
    }

    None
}

#[test]
fn resolve_binds_parameter_name_expression() {
    let source = source(
        r#"
        fn main(value: int32): null {
            var result = value
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "value").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "value");
    assert_eq!(symbol.kind(), SymbolKind::Parameter);
}

#[test]
fn resolve_binds_block_local_name_expression() {
    let source = source(
        r#"
        fn main(): null {
            var first = 1
            var second = first
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "first").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "first");
    assert_eq!(symbol.kind(), SymbolKind::Var);
}

#[test]
fn resolve_binds_name_from_parent_block_scope() {
    let source = source(
        r#"
        fn main(): null {
            var outer = 1

            if true {
                var inner = outer
            }

            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "outer").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "outer");
    assert_eq!(symbol.kind(), SymbolKind::Var);
}

#[test]
fn resolve_prefers_nearest_scope_symbol() {
    let source = source(
        r#"
        fn main(): null {
            var value = 1

            if true {
                var value = 2
                var result = value
            }

            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "value").unwrap();

    let referenced_symbol_id = resolution.reference_symbol(expression).unwrap();
    let referenced_symbol = resolution.symbol(referenced_symbol_id).unwrap();

    let function = syntax.first_child(root).unwrap();

    let outer_block = syntax
        .first_child_of_kind(function, SyntaxNodeKind::Block)
        .unwrap();

    let if_statement = syntax
        .first_child_of_kind(outer_block, SyntaxNodeKind::IfStatement)
        .unwrap();

    let inner_block = syntax
        .first_child_of_kind(if_statement, SyntaxNodeKind::Block)
        .unwrap();

    let inner_block_scope = resolution.node_scope(inner_block).unwrap();

    assert_eq!(referenced_symbol.name(), "value");
    assert_eq!(referenced_symbol.kind(), SymbolKind::Var);
    assert_eq!(referenced_symbol.scope(), inner_block_scope);
}

#[test]
fn resolve_binds_import_namespace_name_expression() {
    let source = source(
        r#"
        import user from "./user"

        fn main(): null {
            var current = user
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "user").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "user");
    assert_eq!(symbol.kind(), SymbolKind::ImportNamespace);
}

#[test]
fn resolve_leaves_unknown_name_expression_unbound_for_now() {
    let source = source(
        r#"
        fn main(): null {
            var value = missing
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    // R7 does not emit unresolved-name diagnostics yet.
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "missing").unwrap();

    assert!(resolution.reference_symbol(expression).is_none());
}
