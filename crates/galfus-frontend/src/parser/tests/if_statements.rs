use super::*;

#[test]
fn parse_if_statement() {
    let source =
        source("fn main(): null {\n  if user.enabled {\n    print(\"enabled\")\n  }\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let if_statement = body_node.children()[0];
    let if_node = syntax.node(if_statement).unwrap();

    assert_eq!(if_node.kind(), SyntaxNodeKind::IfStatement);
    assert_eq!(if_node.children().len(), 2);

    let condition = if_node.children()[0];
    let then_block = if_node.children()[1];

    assert_eq!(
        syntax.node(condition).unwrap().kind(),
        SyntaxNodeKind::MemberExpression
    );

    assert_eq!(
        source.slice(syntax.node(condition).unwrap().span()),
        Some("user.enabled")
    );

    assert_eq!(
        syntax.node(then_block).unwrap().kind(),
        SyntaxNodeKind::Block
    );
}

#[test]
fn parse_if_else_statement() {
    let source = source(
        "fn main(): null {\n  if enabled {\n    print(\"yes\")\n  } else {\n    print(\"no\")\n  }\n  return\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let if_statement = body_node.children()[0];
    let if_node = syntax.node(if_statement).unwrap();

    assert_eq!(if_node.kind(), SyntaxNodeKind::IfStatement);
    assert_eq!(if_node.children().len(), 3);

    let else_clause = if_node.children()[2];
    let else_node = syntax.node(else_clause).unwrap();

    assert_eq!(else_node.kind(), SyntaxNodeKind::ElseClause);
    assert_eq!(else_node.children().len(), 1);

    let else_child = else_node.children()[0];

    assert_eq!(
        syntax.node(else_child).unwrap().kind(),
        SyntaxNodeKind::Block
    );
}

#[test]
fn parse_else_if_statement() {
    let source = source(
        "fn main(): null {\n  if a {\n    print(\"a\")\n  } else if b {\n    print(\"b\")\n  } else {\n    print(\"c\")\n  }\n  return\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let if_statement = body_node.children()[0];
    let if_node = syntax.node(if_statement).unwrap();

    assert_eq!(if_node.kind(), SyntaxNodeKind::IfStatement);
    assert_eq!(if_node.children().len(), 3);

    let else_clause = if_node.children()[2];
    let else_node = syntax.node(else_clause).unwrap();

    assert_eq!(else_node.kind(), SyntaxNodeKind::ElseClause);

    let nested_if = else_node.children()[0];
    let nested_if_node = syntax.node(nested_if).unwrap();

    assert_eq!(nested_if_node.kind(), SyntaxNodeKind::IfStatement);
    assert_eq!(nested_if_node.children().len(), 3);

    let final_else = nested_if_node.children()[2];
    let final_else_node = syntax.node(final_else).unwrap();

    assert_eq!(final_else_node.kind(), SyntaxNodeKind::ElseClause);

    let final_else_child = final_else_node.children()[0];

    assert_eq!(
        syntax.node(final_else_child).unwrap().kind(),
        SyntaxNodeKind::Block
    );
}

#[test]
fn parse_if_else_allows_newline_before_else() {
    let source = source(
        "fn main(): null {\n  if enabled {\n    print(\"yes\")\n  }\n  else {\n    print(\"no\")\n  }\n  return\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());
}

#[test]
fn parse_if_statement_with_binary_condition() {
    let source = source(
        "fn main(): null {\n  if user != null && user.enabled {\n    print(\"ok\")\n  }\n  return\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let if_statement = body_node.children()[0];
    let if_node = syntax.node(if_statement).unwrap();

    let condition = if_node.children()[0];

    assert_eq!(
        syntax.node(condition).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );

    assert_eq!(
        source.slice(syntax.node(condition).unwrap().span()),
        Some("user != null && user.enabled")
    );
}
