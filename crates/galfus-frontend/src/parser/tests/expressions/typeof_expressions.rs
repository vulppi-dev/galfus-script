use super::super::*;

#[test]
fn parse_typeof_expression_with_type_patterns() {
    let source = source(
        "fn main(): [uint8] {\n  return typeof T {\n    int => \"int\",\n    [uint8] => \"bytes\",\n    User => \"user\",\n    _ => \"other\",\n  }\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors(), "{:?}", result.diagnostics());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let expression = find_first_of_kind(syntax, root, SyntaxNodeKind::TypeofExpression).unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::TypeofExpression);

    let subject = expression_node.first_child().unwrap();
    let arms = expression_node.child(1).unwrap();

    assert_eq!(
        syntax.node(subject).unwrap().kind(),
        SyntaxNodeKind::NamedType
    );

    let arms_node = syntax.node(arms).unwrap();
    assert_eq!(arms_node.kind(), SyntaxNodeKind::TypeofArmList);
    assert_eq!(arms_node.child_count(), 4);

    let first_arm = arms_node.first_child().unwrap();
    let first_pattern = syntax.node(first_arm).unwrap().first_child().unwrap();

    assert_eq!(
        syntax.node(first_pattern).unwrap().kind(),
        SyntaxNodeKind::TypePattern
    );
    assert_eq!(
        source.slice(syntax.node(first_pattern).unwrap().span()),
        Some("int")
    );
}
