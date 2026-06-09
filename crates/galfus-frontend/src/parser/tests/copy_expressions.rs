use super::*;

#[test]
fn parse_copy_expression() {
    let source = source("fn main(): null {\n  const clone = copy value\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let initializer = syntax.node(statement).unwrap().children()[1];
    let expression = syntax.node(initializer).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CopyExpression);
    assert_eq!(source.slice(expression_node.span()), Some("copy value"));

    let value = expression_node.children()[0];

    assert_eq!(
        syntax.node(value).unwrap().kind(),
        SyntaxNodeKind::NameExpression
    );
}

#[test]
fn parse_copy_expression_with_member_expression() {
    let source = source("fn main(): null {\n  const clone = copy user.profile\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let initializer = syntax.node(statement).unwrap().children()[1];
    let expression = syntax.node(initializer).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CopyExpression);
    assert_eq!(
        source.slice(expression_node.span()),
        Some("copy user.profile")
    );

    let value = expression_node.children()[0];

    assert_eq!(
        syntax.node(value).unwrap().kind(),
        SyntaxNodeKind::MemberExpression
    );
}

#[test]
fn parse_copy_expression_as_call_argument() {
    let source = source("fn main(): null {\n  send(copy message)\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let expression = syntax.node(statement).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    let arguments = expression_node.children()[1];
    let argument = syntax.node(arguments).unwrap().children()[0];
    let argument_node = syntax.node(argument).unwrap();

    let value = argument_node.children()[0];
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::CopyExpression);
    assert_eq!(source.slice(value_node.span()), Some("copy message"));
}

#[test]
fn parse_copy_expression_has_unary_precedence() {
    let source = source("fn main(): null {\n  const result = copy value + 1\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let initializer = syntax.node(statement).unwrap().children()[1];
    let expression = syntax.node(initializer).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);

    let left = expression_node.children()[0];

    assert_eq!(
        syntax.node(left).unwrap().kind(),
        SyntaxNodeKind::CopyExpression
    );

    assert_eq!(
        source.slice(syntax.node(left).unwrap().span()),
        Some("copy value")
    );
}

#[test]
fn parse_copy_grouped_expression() {
    let source = source("fn main(): null {\n  const result = copy (value + 1)\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let initializer = syntax.node(statement).unwrap().children()[1];
    let expression = syntax.node(initializer).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CopyExpression);

    let value = expression_node.children()[0];

    assert_eq!(
        syntax.node(value).unwrap().kind(),
        SyntaxNodeKind::GroupedExpression
    );

    assert_eq!(
        source.slice(expression_node.span()),
        Some("copy (value + 1)")
    );
}
