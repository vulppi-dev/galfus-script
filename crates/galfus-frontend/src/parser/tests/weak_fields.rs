use super::*;

#[test]
fn parse_weak_struct_field() {
    let source = source("struct CacheEntry {\n  weak resource: Resource | null = null,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    assert_eq!(struct_node.kind(), SyntaxNodeKind::StructItem);

    let fields = struct_node.children()[1];
    let fields_node = syntax.node(fields).unwrap();

    assert_eq!(fields_node.children().len(), 1);

    let field = fields_node.children()[0];
    let field_node = syntax.node(field).unwrap();

    assert_eq!(field_node.kind(), SyntaxNodeKind::WeakStructField);
    assert_eq!(
        source.slice(field_node.span()),
        Some("weak resource: Resource | null = null")
    );

    assert_eq!(field_node.children().len(), 3);

    let name = field_node.children()[0];
    let field_type = field_node.children()[1];
    let default = field_node.children()[2];

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("resource")
    );

    assert_eq!(
        syntax.node(field_type).unwrap().kind(),
        SyntaxNodeKind::UnionType
    );

    assert_eq!(
        syntax.node(default).unwrap().kind(),
        SyntaxNodeKind::StructFieldDefault
    );
}
