use super::super::*;

#[test]
fn parse_while_statement() {
    let source = source("fn main(): null {\n  while running {\n    tick()\n  }\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let while_statement = body_node.first_child().unwrap();
    let while_node = syntax.node(while_statement).unwrap();

    assert_eq!(while_node.kind(), SyntaxNodeKind::WhileStatement);
    assert_eq!(while_node.child_count(), 2);

    let condition = while_node.first_child().unwrap();
    let while_body = while_node.child(1).unwrap();

    assert_eq!(
        syntax.node(condition).unwrap().kind(),
        SyntaxNodeKind::NameExpression
    );

    assert_eq!(
        source.slice(syntax.node(condition).unwrap().span()),
        Some("running")
    );

    assert_eq!(
        syntax.node(while_body).unwrap().kind(),
        SyntaxNodeKind::Block
    );
}

#[test]
fn parse_while_statement_with_binary_condition() {
    let source = source("fn main(): null {\n  while index < count {\n    index += 1\n  }\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let while_statement = body_node.first_child().unwrap();
    let while_node = syntax.node(while_statement).unwrap();

    let condition = while_node.first_child().unwrap();
    let condition_node = syntax.node(condition).unwrap();

    assert_eq!(condition_node.kind(), SyntaxNodeKind::BinaryExpression);
    assert_eq!(source.slice(condition_node.span()), Some("index < count"));

    let while_body = while_node.child(1).unwrap();
    let while_body_node = syntax.node(while_body).unwrap();

    let assignment = while_body_node.first_child().unwrap();

    assert_eq!(
        syntax.node(assignment).unwrap().kind(),
        SyntaxNodeKind::AssignmentStatement
    );
}

#[test]
fn parse_while_statement_with_break_and_continue() {
    let source = source(
        "fn main(): null {\n  while running {\n    if paused {\n      continue\n    }\n    if done {\n      break\n    }\n    tick()\n  }\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let while_statement = body_node.first_child().unwrap();
    let while_node = syntax.node(while_statement).unwrap();

    let while_body = while_node.child(1).unwrap();
    let while_body_node = syntax.node(while_body).unwrap();

    assert_eq!(while_body_node.child_count(), 3);

    let first_if = while_body_node.first_child().unwrap();
    let second_if = while_body_node.child(1).unwrap();
    let tick_statement = while_body_node.child(2).unwrap();

    assert_eq!(
        syntax.node(first_if).unwrap().kind(),
        SyntaxNodeKind::IfStatement
    );

    assert_eq!(
        syntax.node(second_if).unwrap().kind(),
        SyntaxNodeKind::IfStatement
    );

    assert_eq!(
        syntax.node(tick_statement).unwrap().kind(),
        SyntaxNodeKind::ExpressionStatement
    );

    let first_if_block = syntax.node(first_if).unwrap().child(1).unwrap();
    let first_if_block_node = syntax.node(first_if_block).unwrap();

    let continue_statement = first_if_block_node.first_child().unwrap();

    assert_eq!(
        syntax.node(continue_statement).unwrap().kind(),
        SyntaxNodeKind::ContinueStatement
    );

    let second_if_block = syntax.node(second_if).unwrap().child(1).unwrap();
    let second_if_block_node = syntax.node(second_if_block).unwrap();

    let break_statement = second_if_block_node.first_child().unwrap();

    assert_eq!(
        syntax.node(break_statement).unwrap().kind(),
        SyntaxNodeKind::BreakStatement
    );
}

#[test]
fn parse_while_statement_allows_newlines_between_parts() {
    let source = source("fn main(): null {\n  while\n    running\n  {\n    tick()\n  }\n}");

    let result = parse(&source);

    assert!(!result.has_errors());
}

#[test]
fn parse_while_condition_identifier_does_not_become_struct_literal() {
    let source = source("fn main(): null {\n  while running {\n    tick()\n  }\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let while_statement = body_node.first_child().unwrap();
    let while_node = syntax.node(while_statement).unwrap();

    let condition = while_node.first_child().unwrap();

    assert_eq!(
        syntax.node(condition).unwrap().kind(),
        SyntaxNodeKind::NameExpression
    );

    assert_eq!(
        source.slice(syntax.node(condition).unwrap().span()),
        Some("running")
    );
}

#[test]
fn parse_while_condition_allows_struct_literal_inside_call_argument() {
    let source = source(
        "fn main(): null {\n  while isValid(new(User) { permission }) {\n    tick()\n  }\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let while_statement = body_node.first_child().unwrap();
    let while_node = syntax.node(while_statement).unwrap();

    let condition = while_node.first_child().unwrap();
    let condition_node = syntax.node(condition).unwrap();

    assert_eq!(condition_node.kind(), SyntaxNodeKind::CallExpression);
    assert_eq!(
        source.slice(condition_node.span()),
        Some("isValid(new(User) { permission })")
    );

    let arguments = condition_node.child(1).unwrap();
    let argument = syntax.node(arguments).unwrap().first_child().unwrap();
    let argument_node = syntax.node(argument).unwrap();

    let value = argument_node.first_child().unwrap();
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::StructLiteral);
    assert_eq!(
        source.slice(value_node.span()),
        Some("new(User) { permission }")
    );
}

#[test]
fn parse_while_statement_requires_block() {
    let source = source("fn main(): null {\n  while running tick()\n}");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message() == "expected `LeftBrace`, found `Identifier`")
        .expect("missing expected block diagnostic");

    assert_eq!(diagnostic.code().as_str(), "P0001");
}
