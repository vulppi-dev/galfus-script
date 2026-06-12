use super::super::*;

#[test]
fn parse_spread_call_argument() {
    let source = source("fn main(): null {\n  printAll(...values)\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.first_child().unwrap();
    let statement_node = syntax.node(statement).unwrap();

    let expression = statement_node.first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CallExpression);

    let arguments = expression_node.child(1).unwrap();
    let arguments_node = syntax.node(arguments).unwrap();

    assert_eq!(arguments_node.child_count(), 1);

    let argument = arguments_node.first_child().unwrap();
    let argument_node = syntax.node(argument).unwrap();

    assert_eq!(argument_node.kind(), SyntaxNodeKind::SpreadArgument);
    assert_eq!(source.slice(argument_node.span()), Some("...values"));

    let value = argument_node.first_child().unwrap();

    assert_eq!(
        syntax.node(value).unwrap().kind(),
        SyntaxNodeKind::NameExpression
    );
}

#[test]
fn parse_mixed_call_arguments_with_spread() {
    let source = source("fn main(): null {\n  call(1, ...values, 3)\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.first_child().unwrap();
    let statement_node = syntax.node(statement).unwrap();

    let expression = statement_node.first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    let arguments = expression_node.child(1).unwrap();
    let arguments_node = syntax.node(arguments).unwrap();

    assert_eq!(arguments_node.child_count(), 3);

    let first = arguments_node.first_child().unwrap();
    let second = arguments_node.child(1).unwrap();
    let third = arguments_node.child(2).unwrap();

    assert_eq!(syntax.node(first).unwrap().kind(), SyntaxNodeKind::Argument);
    assert_eq!(
        syntax.node(second).unwrap().kind(),
        SyntaxNodeKind::SpreadArgument
    );
    assert_eq!(syntax.node(third).unwrap().kind(), SyntaxNodeKind::Argument);

    assert_eq!(
        source.slice(syntax.node(second).unwrap().span()),
        Some("...values")
    );
}

#[test]
fn parse_spread_array_element() {
    let source = source("fn main(): null {\n  const all = [1, ...rest]\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.first_child().unwrap();
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.child(1).unwrap();
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::ArrayLiteral);
    assert_eq!(source.slice(expression_node.span()), Some("[1, ...rest]"));
    assert_eq!(expression_node.child_count(), 2);

    let first = expression_node.first_child().unwrap();
    let second = expression_node.child(1).unwrap();

    assert_eq!(
        syntax.node(first).unwrap().kind(),
        SyntaxNodeKind::ArrayElement
    );
    assert_eq!(
        syntax.node(second).unwrap().kind(),
        SyntaxNodeKind::SpreadArrayElement
    );

    assert_eq!(
        source.slice(syntax.node(second).unwrap().span()),
        Some("...rest")
    );
}

#[test]
fn parse_spread_array_element_allows_newline_after_spread() {
    let source = source("fn main(): null {\n  const all = [...\n    rest]\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.first_child().unwrap();
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.child(1).unwrap();
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    let element = expression_node.first_child().unwrap();
    let element_node = syntax.node(element).unwrap();

    assert_eq!(element_node.kind(), SyntaxNodeKind::SpreadArrayElement);

    assert_eq!(source.slice(element_node.span()), Some("...\n    rest"));
}

#[test]
fn parse_spread_call_argument_accepts_trailing_comma() {
    let source = source("fn main(): null {\n  printAll(...values,)\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.first_child().unwrap();
    let statement_node = syntax.node(statement).unwrap();

    let expression = statement_node.first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    let arguments = expression_node.child(1).unwrap();
    let arguments_node = syntax.node(arguments).unwrap();

    assert_eq!(arguments_node.child_count(), 1);

    let argument = arguments_node.first_child().unwrap();

    assert_eq!(
        syntax.node(argument).unwrap().kind(),
        SyntaxNodeKind::SpreadArgument
    );
}

#[test]
fn parse_spread_is_not_valid_standalone_expression() {
    let source = source("fn main(): null {\n  return ...values\n}");

    let result = parse(&source);

    assert!(result.has_errors());
}
