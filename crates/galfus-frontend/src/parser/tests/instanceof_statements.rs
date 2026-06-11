use super::*;

#[test]
fn parse_instanceof_statement_with_type_patterns() {
    let source = source(
        "fn main(): int32 {\n  instanceof value {\n    int32(v) => {\n      return v ** 2\n    }\n    String(text) => {\n      return text.length\n    }\n    _ => {\n      return 0\n    }\n  }\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let statement_node = syntax.node(statement).unwrap();

    assert_eq!(statement_node.kind(), SyntaxNodeKind::InstanceofStatement);

    let subject = statement_node.first_child().unwrap();
    let arms = statement_node.child(1).unwrap();

    assert_eq!(
        syntax.node(subject).unwrap().kind(),
        SyntaxNodeKind::NameExpression
    );

    let arms_node = syntax.node(arms).unwrap();

    assert_eq!(arms_node.kind(), SyntaxNodeKind::InstanceofArmList);

    assert_eq!(arms_node.child_count(), 3);

    let first_arm = arms_node.first_child().unwrap();
    let first_arm_node = syntax.node(first_arm).unwrap();

    let first_pattern = first_arm_node.first_child().unwrap();
    let first_pattern_node = syntax.node(first_pattern).unwrap();

    assert_eq!(first_pattern_node.kind(), SyntaxNodeKind::TypePattern);
    assert_eq!(source.slice(first_pattern_node.span()), Some("int32(v)"));
}

#[test]
fn parse_instanceof_fallback_as_binding_pattern() {
    let source = source(
        "fn main(): int32 {\n  instanceof value {\n    _ => {\n      return 0\n    }\n  }\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let statement_node = syntax.node(statement).unwrap();

    let arms = statement_node.child(1).unwrap();
    let arm = syntax.node(arms).unwrap().first_child().unwrap();
    let arm_node = syntax.node(arm).unwrap();

    let pattern = arm_node.first_child().unwrap();
    let pattern_node = syntax.node(pattern).unwrap();

    assert_eq!(pattern_node.kind(), SyntaxNodeKind::BindingPattern);
    assert_eq!(source.slice(pattern_node.span()), Some("_"));
}
