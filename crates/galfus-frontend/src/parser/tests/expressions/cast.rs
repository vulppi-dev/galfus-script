use super::super::*;

#[test]
fn parse_cast_expression() {
    let source = source("var a = <i8> 6.24");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();

    let var_item = syntax.first_child(root).unwrap();

    let initializer = syntax
        .first_child_of_kind(var_item, SyntaxNodeKind::Initializer)
        .unwrap();

    let expression = syntax.first_child(initializer).unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CastExpression);
    assert_eq!(expression_node.child_count(), 2);

    let ty = syntax.child(expression, 0).unwrap();
    let value = syntax.child(expression, 1).unwrap();

    assert!(syntax.node(ty).unwrap().kind().is_type());
    assert_eq!(
        syntax.node(value).unwrap().kind(),
        SyntaxNodeKind::FloatLiteral
    );
}

#[test]
fn parse_cast_expression_with_path_type() {
    let source = source("var a = <collections::Id> value");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();

    let var_item = syntax.first_child(root).unwrap();

    let initializer = syntax
        .first_child_of_kind(var_item, SyntaxNodeKind::Initializer)
        .unwrap();

    let expression = syntax.first_child(initializer).unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CastExpression);

    let ty = syntax.child(expression, 0).unwrap();

    assert_eq!(syntax.node(ty).unwrap().kind(), SyntaxNodeKind::Path);
    assert_eq!(
        source.slice(syntax.node(ty).unwrap().span()),
        Some("collections::Id")
    );
}

#[test]
fn parse_cast_expression_as_unary_operand() {
    let source = source("var a = -<i32> value");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();

    let var_item = syntax.first_child(root).unwrap();

    let initializer = syntax
        .first_child_of_kind(var_item, SyntaxNodeKind::Initializer)
        .unwrap();

    let expression = syntax.first_child(initializer).unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::UnaryExpression);

    let operand = syntax.child(expression, 1).unwrap();

    assert_eq!(
        syntax.node(operand).unwrap().kind(),
        SyntaxNodeKind::CastExpression
    );
}

#[test]
fn parse_cast_expression_inside_binary_expression() {
    let source = source("var a = <i32> value + 1");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();

    let var_item = syntax.first_child(root).unwrap();

    let initializer = syntax
        .first_child_of_kind(var_item, SyntaxNodeKind::Initializer)
        .unwrap();

    let expression = syntax.first_child(initializer).unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);

    let left = syntax.child(expression, 0).unwrap();

    assert_eq!(
        syntax.node(left).unwrap().kind(),
        SyntaxNodeKind::CastExpression
    );
}
