use super::*;

#[test]
fn parse_arrow_function_expression_body() {
    let source = source(
        "fn main(): null {\n  const double = (value: int32): int32 => value * 2\n  return\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(
        expression_node.kind(),
        SyntaxNodeKind::ArrowFunctionExpression
    );

    assert_eq!(
        source.slice(expression_node.span()),
        Some("(value: int32): int32 => value * 2")
    );

    assert_eq!(expression_node.children().len(), 3);

    let parameters = expression_node.children()[0];
    let return_type = expression_node.children()[1];
    let arrow_body = expression_node.children()[2];

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
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let initializer = syntax.node(statement).unwrap().children()[1];
    let expression = syntax.node(initializer).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(
        expression_node.kind(),
        SyntaxNodeKind::ArrowFunctionExpression
    );

    assert_eq!(expression_node.children().len(), 2);

    let parameters = expression_node.children()[0];
    let arrow_body = expression_node.children()[1];

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
        "fn main(): null {\n  const printer = (value: String): null => {\n    print(value)\n    return\n  }\n  return\n}",
    );

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

    assert_eq!(
        expression_node.kind(),
        SyntaxNodeKind::ArrowFunctionExpression
    );

    let arrow_body = expression_node.children()[2];

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
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let initializer = syntax.node(statement).unwrap().children()[1];
    let expression = syntax.node(initializer).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    let parameters = expression_node.children()[0];
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.children().len(), 1);

    let parameter = parameters_node.children()[0];
    let parameter_node = syntax.node(parameter).unwrap();

    assert_eq!(parameter_node.kind(), SyntaxNodeKind::RestParameter);
    assert_eq!(parameter_node.children().len(), 3);
}

#[test]
fn parse_grouped_expression_still_works() {
    let source = source("fn main(): null {\n  const value = (1 + 2) * 3\n  return\n}");

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
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let expression = syntax.node(statement).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CallExpression);

    let arguments = expression_node.children()[1];
    let argument = syntax.node(arguments).unwrap().children()[0];
    let argument_node = syntax.node(argument).unwrap();

    let value = argument_node.children()[0];
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::ArrowFunctionExpression);
}
