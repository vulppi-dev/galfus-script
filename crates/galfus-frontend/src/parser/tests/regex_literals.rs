use super::*;

#[test]
fn parse_regex_literal_with_escaped_slash() {
    let source = source("fn main(): null {\n  const pattern = /a\\/b/\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());
}

#[test]
fn parse_regex_literal_with_character_class() {
    let source = source("fn main(): null {\n  const pattern = /[a-z]+/i\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());
}

#[test]
fn parse_slash_after_expression_as_division() {
    let source = source("fn main(): null {\n  const value = a / b\n  return\n}");

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

    assert_eq!(
        syntax.node(expression).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );
}

#[test]
fn parse_regex_pattern_in_match_statement() {
    let source = source(
        "fn main(): null {\n  match text {\n    /^admin-/ => {\n      return\n    }\n    _ => {\n      return\n    }\n  }\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let match_statement = syntax.node(body).unwrap().children()[0];
    let match_node = syntax.node(match_statement).unwrap();

    let arms = match_node.children()[1];
    let first_arm = syntax.node(arms).unwrap().children()[0];
    let first_arm_node = syntax.node(first_arm).unwrap();

    let pattern = first_arm_node.children()[0];
    let pattern_node = syntax.node(pattern).unwrap();

    assert_eq!(pattern_node.kind(), SyntaxNodeKind::RegexPattern);
    assert_eq!(source.slice(pattern_node.span()), Some("/^admin-/"));

    assert_eq!(pattern_node.children().len(), 1);

    let regex = pattern_node.children()[0];
    let regex_node = syntax.node(regex).unwrap();

    assert_eq!(regex_node.kind(), SyntaxNodeKind::RegexLiteral);
    assert_eq!(source.slice(regex_node.span()), Some("/^admin-/"));
}

#[test]
fn parse_regex_pattern_with_flags_in_match_statement() {
    let source = source(
        "fn main(): null {\n  match email {\n    /@gmail\\.com$/i => {\n      return\n    }\n    _ => {\n      return\n    }\n  }\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let match_statement = syntax.node(body).unwrap().children()[0];
    let match_node = syntax.node(match_statement).unwrap();

    let arms = match_node.children()[1];
    let first_arm = syntax.node(arms).unwrap().children()[0];
    let first_arm_node = syntax.node(first_arm).unwrap();

    let pattern = first_arm_node.children()[0];
    let pattern_node = syntax.node(pattern).unwrap();

    assert_eq!(pattern_node.kind(), SyntaxNodeKind::RegexPattern);
    assert_eq!(source.slice(pattern_node.span()), Some("/@gmail\\.com$/i"));
}

#[test]
fn parse_string_literal_pattern_still_uses_literal_pattern() {
    let source = source(
        "fn main(): null {\n  match status {\n    \"active\" => {\n      return\n    }\n    _ => {\n      return\n    }\n  }\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let match_statement = syntax.node(body).unwrap().children()[0];
    let match_node = syntax.node(match_statement).unwrap();

    let arms = match_node.children()[1];
    let first_arm = syntax.node(arms).unwrap().children()[0];
    let first_arm_node = syntax.node(first_arm).unwrap();

    let pattern = first_arm_node.children()[0];
    let pattern_node = syntax.node(pattern).unwrap();

    assert_eq!(pattern_node.kind(), SyntaxNodeKind::LiteralPattern);

    let literal = pattern_node.children()[0];

    assert_eq!(
        syntax.node(literal).unwrap().kind(),
        SyntaxNodeKind::StringLiteral
    );
}

#[test]
fn parse_regex_literal_expression() {
    let source = source("fn main(): null {\n  const pattern = /abc/i\n  return\n}");

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

    assert_eq!(expression_node.kind(), SyntaxNodeKind::RegexLiteral);
    assert_eq!(source.slice(expression_node.span()), Some("/abc/i"));
}
