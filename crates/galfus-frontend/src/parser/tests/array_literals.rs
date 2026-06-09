use super::*;

#[test]
fn parse_empty_array_literal() {
    let source = source("fn main(): null {\n  const values = []\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.children()[0];
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::ArrayLiteral);
    assert_eq!(source.slice(expression_node.span()), Some("[]"));
    assert!(expression_node.children().is_empty());
}

#[test]
fn parse_array_literal_with_elements() {
    let source = source("fn main(): null {\n  const values = [1, 2, 3]\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.children()[0];
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::ArrayLiteral);
    assert_eq!(source.slice(expression_node.span()), Some("[1, 2, 3]"));
    assert_eq!(expression_node.children().len(), 3);

    let first_element = expression_node.children()[0];
    let first_element_node = syntax.node(first_element).unwrap();

    assert_eq!(first_element_node.kind(), SyntaxNodeKind::ArrayElement);

    let first_value = first_element_node.children()[0];

    assert_eq!(
        syntax.node(first_value).unwrap().kind(),
        SyntaxNodeKind::IntegerLiteral
    );
    assert_eq!(
        source.slice(syntax.node(first_value).unwrap().span()),
        Some("1")
    );
}

#[test]
fn parse_array_literal_accepts_trailing_comma() {
    let source = source("fn main(): null {\n  const values = [1, 2, 3,]\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.children()[0];
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::ArrayLiteral);
    assert_eq!(expression_node.children().len(), 3);
    assert_eq!(source.slice(expression_node.span()), Some("[1, 2, 3,]"));
}

#[test]
fn parse_array_literal_allows_internal_newlines() {
    let source =
        source("fn main(): null {\n  const values = [\n    1,\n    2,\n    3,\n  ]\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.children()[0];
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::ArrayLiteral);
    assert_eq!(expression_node.children().len(), 3);
    assert_eq!(
        source.slice(expression_node.span()),
        Some("[\n    1,\n    2,\n    3,\n  ]")
    );
}

#[test]
fn parse_nested_array_literal() {
    let source = source("fn main(): null {\n  const values = [[1], [2]]\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.children()[0];
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::ArrayLiteral);
    assert_eq!(source.slice(expression_node.span()), Some("[[1], [2]]"));
    assert_eq!(expression_node.children().len(), 2);

    let first_element = expression_node.children()[0];
    let first_element_node = syntax.node(first_element).unwrap();

    let first_value = first_element_node.children()[0];

    assert_eq!(
        syntax.node(first_value).unwrap().kind(),
        SyntaxNodeKind::ArrayLiteral
    );

    assert_eq!(
        source.slice(syntax.node(first_value).unwrap().span()),
        Some("[1]")
    );
}

#[test]
fn parse_array_literal_with_expression_elements() {
    let source =
        source("fn main(): null {\n  const values = [1 + 2, user.score, getValue()]\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let statement = body_node.children()[0];
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::ArrayLiteral);
    assert_eq!(expression_node.children().len(), 3);

    let first_element = expression_node.children()[0];
    let second_element = expression_node.children()[1];
    let third_element = expression_node.children()[2];

    let first_value = syntax.node(first_element).unwrap().children()[0];
    let second_value = syntax.node(second_element).unwrap().children()[0];
    let third_value = syntax.node(third_element).unwrap().children()[0];

    assert_eq!(
        syntax.node(first_value).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );

    assert_eq!(
        syntax.node(second_value).unwrap().kind(),
        SyntaxNodeKind::MemberExpression
    );

    assert_eq!(
        syntax.node(third_value).unwrap().kind(),
        SyntaxNodeKind::CallExpression
    );
}
