use super::*;

#[test]
fn parse_binary_addition_expression() {
    let source = source("fn main(): int32 {\n  return 1 + 2\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("1 + 2"));

    let left = expression_node.children()[0];
    let operator = expression_node.children()[1];
    let right = expression_node.children()[2];

    assert_eq!(
        syntax.node(left).unwrap().kind(),
        SyntaxNodeKind::IntegerLiteral
    );
    assert_eq!(source.slice(syntax.node(left).unwrap().span()), Some("1"));

    assert_eq!(
        syntax.node(operator).unwrap().kind(),
        SyntaxNodeKind::BinaryOperator
    );
    assert_eq!(
        source.slice(syntax.node(operator).unwrap().span()),
        Some("+")
    );

    assert_eq!(
        syntax.node(right).unwrap().kind(),
        SyntaxNodeKind::IntegerLiteral
    );
    assert_eq!(source.slice(syntax.node(right).unwrap().span()), Some("2"));
}

#[test]
fn parse_binary_expression_respects_precedence() {
    let source = source("fn main(): int32 {\n  return 1 + 2 * 3\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("1 + 2 * 3"));

    let root_operator = expression_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(root_operator).unwrap().span()),
        Some("+")
    );

    let right = expression_node.children()[2];
    let right_node = syntax.node(right).unwrap();

    assert_eq!(right_node.kind(), SyntaxNodeKind::BinaryExpression);

    let right_operator = right_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(right_operator).unwrap().span()),
        Some("*")
    );
}

#[test]
fn parse_power_expression_is_right_associative() {
    let source = source("fn main(): int32 {\n  return 2 ** 3 ** 4\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("2 ** 3 ** 4"));

    let root_operator = expression_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(root_operator).unwrap().span()),
        Some("**")
    );

    let right = expression_node.children()[2];
    let right_node = syntax.node(right).unwrap();

    assert_eq!(right_node.kind(), SyntaxNodeKind::BinaryExpression);

    let right_operator = right_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(right_operator).unwrap().span()),
        Some("**")
    );
}

#[test]
fn parse_binary_expression_allows_newline_before_operator() {
    let source = source("fn main(): int32 {\n  return 1\n  + 2\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("1\n  + 2"));
}

#[test]
fn parse_binary_expression_allows_newline_after_operator() {
    let source = source("fn main(): int32 {\n  return 1 +\n  2\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("1 +\n  2"));
}

#[test]
fn parse_comparison_expression_after_arithmetic() {
    let source = source("fn main(): bool {\n  return 1 + 2 > 3\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("1 + 2 > 3"));

    let root_operator = expression_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(root_operator).unwrap().span()),
        Some(">")
    );

    let left = expression_node.children()[0];
    let left_node = syntax.node(left).unwrap();

    assert_eq!(left_node.kind(), SyntaxNodeKind::BinaryExpression);

    let left_operator = left_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(left_operator).unwrap().span()),
        Some("+")
    );
}

#[test]
fn parse_logical_and_has_higher_precedence_than_or() {
    let source = source("fn main(): bool {\n  return a || b && c\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("a || b && c"));

    let root_operator = expression_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(root_operator).unwrap().span()),
        Some("||")
    );

    let right = expression_node.children()[2];
    let right_node = syntax.node(right).unwrap();

    assert_eq!(right_node.kind(), SyntaxNodeKind::BinaryExpression);

    let right_operator = right_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(right_operator).unwrap().span()),
        Some("&&")
    );
}

#[test]
fn parse_equality_has_higher_precedence_than_logical_and() {
    let source = source("fn main(): bool {\n  return a == b && c != d\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(
        source.slice(expression_node.span()),
        Some("a == b && c != d")
    );

    let root_operator = expression_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(root_operator).unwrap().span()),
        Some("&&")
    );

    let left = expression_node.children()[0];
    let right = expression_node.children()[2];

    assert_eq!(
        syntax.node(left).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );
    assert_eq!(
        syntax.node(right).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );
}

#[test]
fn parse_null_coalescing_is_right_associative() {
    let source = source("fn main(): User {\n  return a ?? b ?? c\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("a ?? b ?? c"));

    let root_operator = expression_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(root_operator).unwrap().span()),
        Some("??")
    );

    let right = expression_node.children()[2];
    let right_node = syntax.node(right).unwrap();

    assert_eq!(right_node.kind(), SyntaxNodeKind::BinaryExpression);

    let right_operator = right_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(right_operator).unwrap().span()),
        Some("??")
    );
}

#[test]
fn parse_logical_expression_allows_newline_before_operator() {
    let source = source("fn main(): bool {\n  return a\n  && b\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("a\n  && b"));
}

#[test]
fn parse_logical_expression_allows_newline_after_operator() {
    let source = source("fn main(): bool {\n  return a &&\n  b\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let expression = return_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(expression_node.span()), Some("a &&\n  b"));
}
