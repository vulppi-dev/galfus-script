use super::*;

#[test]
fn parse_struct_satisfies_single_constraint() {
    let source = source("struct User satisfies Identifiable {\n  id: int64,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    assert_eq!(struct_node.kind(), SyntaxNodeKind::StructItem);
    assert_eq!(struct_node.children().len(), 3);

    let name = struct_node.children()[0];
    let satisfies = struct_node.children()[1];
    let fields = struct_node.children()[2];

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("User")
    );

    let satisfies_node = syntax.node(satisfies).unwrap();

    assert_eq!(satisfies_node.kind(), SyntaxNodeKind::SatisfiesClause);

    assert_eq!(
        source.slice(satisfies_node.span()),
        Some("satisfies Identifiable")
    );

    assert_eq!(satisfies_node.children().len(), 1);

    let constraint = satisfies_node.children()[0];

    assert_eq!(
        syntax.node(constraint).unwrap().kind(),
        SyntaxNodeKind::NamedType
    );

    assert_eq!(
        source.slice(syntax.node(constraint).unwrap().span()),
        Some("Identifiable")
    );

    assert_eq!(
        syntax.node(fields).unwrap().kind(),
        SyntaxNodeKind::StructFieldList
    );
}

#[test]
fn parse_struct_satisfies_multiple_constraints() {
    let source = source(
        "struct User satisfies Identifiable, Stringable {\n  id: int64,\n  name: String,\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    let satisfies = struct_node.children()[1];
    let satisfies_node = syntax.node(satisfies).unwrap();

    assert_eq!(satisfies_node.kind(), SyntaxNodeKind::SatisfiesClause);
    assert_eq!(satisfies_node.children().len(), 2);

    assert_eq!(
        source.slice(satisfies_node.span()),
        Some("satisfies Identifiable, Stringable")
    );

    let first = satisfies_node.children()[0];
    let second = satisfies_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(first).unwrap().span()),
        Some("Identifiable")
    );

    assert_eq!(
        source.slice(syntax.node(second).unwrap().span()),
        Some("Stringable")
    );
}

#[test]
fn parse_struct_satisfies_generic_constraint() {
    let source = source(
        "struct Range satisfies Iterable<int32, RangeIterator> {\n  start: int32,\n  end: int32,\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    let satisfies = struct_node.children()[1];
    let satisfies_node = syntax.node(satisfies).unwrap();

    assert_eq!(satisfies_node.kind(), SyntaxNodeKind::SatisfiesClause);
    assert_eq!(satisfies_node.children().len(), 1);

    let constraint = satisfies_node.children()[0];
    let constraint_node = syntax.node(constraint).unwrap();

    assert_eq!(constraint_node.kind(), SyntaxNodeKind::GenericType);

    assert_eq!(
        source.slice(constraint_node.span()),
        Some("Iterable<int32, RangeIterator>")
    );
}

#[test]
fn parse_generic_struct_with_satisfies_clause() {
    let source = source("struct Box<T> satisfies Container<T> {\n  value: T,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    assert_eq!(struct_node.children().len(), 4);

    let name = struct_node.children()[0];
    let generics = struct_node.children()[1];
    let satisfies = struct_node.children()[2];
    let fields = struct_node.children()[3];

    assert_eq!(source.slice(syntax.node(name).unwrap().span()), Some("Box"));

    assert_eq!(
        syntax.node(generics).unwrap().kind(),
        SyntaxNodeKind::GenericParameterList
    );

    assert_eq!(
        syntax.node(satisfies).unwrap().kind(),
        SyntaxNodeKind::SatisfiesClause
    );

    assert_eq!(
        syntax.node(fields).unwrap().kind(),
        SyntaxNodeKind::StructFieldList
    );
}

#[test]
fn parse_struct_without_satisfies_shape_is_unchanged() {
    let source = source("struct User {\n  name: String,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    assert_eq!(struct_node.children().len(), 2);

    assert_eq!(
        syntax.node(struct_node.children()[1]).unwrap().kind(),
        SyntaxNodeKind::StructFieldList
    );
}
