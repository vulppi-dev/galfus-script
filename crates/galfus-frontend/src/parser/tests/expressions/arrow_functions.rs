use super::super::*;

#[test]
fn parse_arrow_function_expression_body() {
    let source = source(
        "fn main(): null {\n  const double = (value: int32): int32 => value * 2\n  return\n}",
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

    let initializer = statement_node.child(1).unwrap();
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(
        expression_node.kind(),
        SyntaxNodeKind::ArrowFunctionExpression
    );

    assert_eq!(
        source.slice(expression_node.span()),
        Some("(value: int32): int32 => value * 2")
    );

    assert_eq!(expression_node.child_count(), 3);

    let parameters = expression_node.first_child().unwrap();
    let return_type = expression_node.child(1).unwrap();
    let arrow_body = expression_node.child(2).unwrap();

    assert_eq!(
        syntax.node(parameters).unwrap().kind(),
        SyntaxNodeKind::ParameterList
    );

    assert_eq!(
        syntax.node(return_type).unwrap().kind(),
        SyntaxNodeKind::NamedType
    );

    assert_eq!(
        syntax.node(arrow_body).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );
}

#[test]
fn parse_arrow_function_without_return_type() {
    let source =
        source("fn main(): null {\n  const double = (value: int32) => value * 2\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let initializer = syntax.node(statement).unwrap().child(1).unwrap();
    let expression = syntax.node(initializer).unwrap().first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(
        expression_node.kind(),
        SyntaxNodeKind::ArrowFunctionExpression
    );

    assert_eq!(expression_node.child_count(), 2);

    let parameters = expression_node.first_child().unwrap();
    let arrow_body = expression_node.child(1).unwrap();

    assert_eq!(
        syntax.node(parameters).unwrap().kind(),
        SyntaxNodeKind::ParameterList
    );

    assert_eq!(
        syntax.node(arrow_body).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );
}

#[test]
fn parse_arrow_function_block_body() {
    let source = source(
        "fn main(): null {\n  const printer = (value: [int8]): null => {\n    print(value)\n    return\n  }\n  return\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let initializer = syntax.node(statement).unwrap().child(1).unwrap();
    let expression = syntax.node(initializer).unwrap().first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(
        expression_node.kind(),
        SyntaxNodeKind::ArrowFunctionExpression
    );

    let arrow_body = expression_node.child(2).unwrap();

    assert_eq!(
        syntax.node(arrow_body).unwrap().kind(),
        SyntaxNodeKind::Block
    );
}

#[test]
fn parse_arrow_function_with_rest_default_parameter() {
    let source = source(
        "fn main(): null {\n  const summarize = (...values: [int32] | null = null): int32 => 0\n  return\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let initializer = syntax.node(statement).unwrap().child(1).unwrap();
    let expression = syntax.node(initializer).unwrap().first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    let parameters = expression_node.first_child().unwrap();
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.child_count(), 1);

    let parameter = parameters_node.first_child().unwrap();
    let parameter_node = syntax.node(parameter).unwrap();

    assert_eq!(parameter_node.kind(), SyntaxNodeKind::RestParameter);
    assert_eq!(parameter_node.child_count(), 3);
}

#[test]
fn parse_grouped_expression_still_works() {
    let source = source("fn main(): null {\n  const value = (1 + 2) * 3\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let initializer = syntax.node(statement).unwrap().child(1).unwrap();
    let expression = syntax.node(initializer).unwrap().first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);

    let left = expression_node.first_child().unwrap();

    assert_eq!(
        syntax.node(left).unwrap().kind(),
        SyntaxNodeKind::GroupedExpression
    );
}

#[test]
fn parse_arrow_function_as_call_argument() {
    let source =
        source("fn main(): null {\n  items.map((item: int32): int32 => item * 2)\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let expression = syntax.node(statement).unwrap().first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CallExpression);

    let arguments = expression_node.child(1).unwrap();
    let argument = syntax.node(arguments).unwrap().first_child().unwrap();
    let argument_node = syntax.node(argument).unwrap();

    let value = argument_node.first_child().unwrap();
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::ArrowFunctionExpression);
}
