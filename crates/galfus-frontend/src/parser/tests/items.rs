use super::*;

#[test]
fn parse_top_level_const_item() {
    let source = source("const version = 1");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.child_count(), 1);

    let item = root_node.first_child().unwrap();
    let item_node = syntax.node(item).unwrap();

    assert_eq!(item_node.kind(), SyntaxNodeKind::ConstItem);
    assert_eq!(source.slice(item_node.span()), Some("const version = 1"));
}

#[test]
fn parse_top_level_var_item() {
    let source = source("var counter: int32 = 0");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.child_count(), 1);

    let item = root_node.first_child().unwrap();
    let item_node = syntax.node(item).unwrap();

    assert_eq!(item_node.kind(), SyntaxNodeKind::VarItem);
    assert_eq!(
        source.slice(item_node.span()),
        Some("var counter: int32 = 0")
    );
}

#[test]
fn parse_export_const_item() {
    let source = source("export const version = 1");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.child_count(), 1);

    let export = root_node.first_child().unwrap();
    let export_node = syntax.node(export).unwrap();

    assert_eq!(export_node.kind(), SyntaxNodeKind::ExportItem);
    assert_eq!(
        source.slice(export_node.span()),
        Some("export const version = 1")
    );
    assert_eq!(export_node.child_count(), 1);

    let inner = export_node.first_child().unwrap();
    let inner_node = syntax.node(inner).unwrap();

    assert_eq!(inner_node.kind(), SyntaxNodeKind::ConstItem);
    assert_eq!(source.slice(inner_node.span()), Some("const version = 1"));
}

#[test]
fn parse_export_var_item() {
    let source = source("export var counter: int32 = 0");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.child_count(), 1);

    let export = root_node.first_child().unwrap();
    let export_node = syntax.node(export).unwrap();

    assert_eq!(export_node.kind(), SyntaxNodeKind::ExportItem);
    assert_eq!(
        source.slice(export_node.span()),
        Some("export var counter: int32 = 0")
    );
    assert_eq!(export_node.child_count(), 1);

    let inner = export_node.first_child().unwrap();
    let inner_node = syntax.node(inner).unwrap();

    assert_eq!(inner_node.kind(), SyntaxNodeKind::VarItem);
    assert_eq!(
        source.slice(inner_node.span()),
        Some("var counter: int32 = 0")
    );
}

#[test]
fn parse_block_var_and_const_as_statements() {
    let source = source(
        "fn main(): null {
            const version = 1
            var counter = 0
            return
        }",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    let function = root_node.first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node
        .children()
        .iter()
        .copied()
        .find(|child| syntax.node(*child).unwrap().kind() == SyntaxNodeKind::Block)
        .unwrap();

    let body_node = syntax.node(body).unwrap();

    assert_eq!(body_node.child_count(), 3);

    let const_statement = body_node.first_child().unwrap();
    let var_statement = body_node.child(1).unwrap();

    assert_eq!(
        syntax.node(const_statement).unwrap().kind(),
        SyntaxNodeKind::ConstStatement
    );

    assert_eq!(
        syntax.node(var_statement).unwrap().kind(),
        SyntaxNodeKind::VarStatement
    );
}

#[test]
fn parse_const_statement_requires_initializer() {
    let source = source(
        "fn main(): null {
            const version
            return
        }",
    );

    let result = parse(&source);

    assert!(result.has_errors());
}

#[test]
fn parse_const_item_requires_initializer() {
    let source = source("const version");

    let result = parse(&source);

    assert!(result.has_errors());
}

#[test]
fn parse_var_item_allows_missing_initializer() {
    let source = source("var counter: int32");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    let item = root_node.first_child().unwrap();
    let item_node = syntax.node(item).unwrap();

    assert_eq!(item_node.kind(), SyntaxNodeKind::VarItem);
    assert_eq!(source.slice(item_node.span()), Some("var counter: int32"));
}
