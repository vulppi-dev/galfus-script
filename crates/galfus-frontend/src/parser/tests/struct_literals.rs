use super::*;

#[test]
fn parse_empty_struct_literal() {
    let source = source("fn main(): null {\n  const user = User {}\n  return\n}");

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

    assert_eq!(expression_node.kind(), SyntaxNodeKind::StructLiteral);
    assert_eq!(source.slice(expression_node.span()), Some("User {}"));
    assert_eq!(expression_node.children().len(), 2);

    let name = expression_node.children()[0];
    let fields = expression_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("User")
    );

    let fields_node = syntax.node(fields).unwrap();

    assert_eq!(fields_node.kind(), SyntaxNodeKind::StructLiteralFieldList);

    assert!(fields_node.children().is_empty());
}

#[test]
fn parse_struct_literal_with_fields() {
    let source = source(
        "fn main(): null {\n  const user = User {\n    name: \"Ana\",\n    age: 32,\n  }\n  return\n}",
    );

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

    assert_eq!(expression_node.kind(), SyntaxNodeKind::StructLiteral);

    let fields = expression_node.children()[1];
    let fields_node = syntax.node(fields).unwrap();

    assert_eq!(fields_node.kind(), SyntaxNodeKind::StructLiteralFieldList);
    assert_eq!(fields_node.children().len(), 2);

    let first_field = fields_node.children()[0];
    let first_field_node = syntax.node(first_field).unwrap();

    assert_eq!(first_field_node.kind(), SyntaxNodeKind::StructLiteralField);

    assert_eq!(source.slice(first_field_node.span()), Some("name: \"Ana\""));

    let first_name = first_field_node.children()[0];
    let first_value = first_field_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(first_name).unwrap().span()),
        Some("name")
    );
    assert_eq!(
        syntax.node(first_value).unwrap().kind(),
        SyntaxNodeKind::StringLiteral
    );
}

#[test]
fn parse_struct_literal_field_value_can_be_expression() {
    let source = source(
        "fn main(): null {\n  const user = User {\n    age: currentAge + 1,\n  }\n  return\n}",
    );

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

    let fields = expression_node.children()[1];
    let field = syntax.node(fields).unwrap().children()[0];
    let field_node = syntax.node(field).unwrap();

    let value = field_node.children()[1];

    assert_eq!(
        syntax.node(value).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );

    assert_eq!(
        source.slice(syntax.node(value).unwrap().span()),
        Some("currentAge + 1")
    );
}

#[test]
fn parse_nested_struct_literal() {
    let source = source(
        "fn main(): null {\n  const user = User {\n    address: Address {\n      city: \"Recife\",\n    },\n  }\n  return\n}",
    );

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

    let fields = expression_node.children()[1];
    let field = syntax.node(fields).unwrap().children()[0];
    let field_node = syntax.node(field).unwrap();

    let value = field_node.children()[1];
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::StructLiteral);
    assert_eq!(
        source.slice(value_node.span()),
        Some("Address {\n      city: \"Recife\",\n    }")
    );
}

#[test]
fn parse_struct_literal_requires_commas_between_fields() {
    let source = source(
        "fn main(): null {\n  const user = User {\n    name: \"Ana\"\n    age: 32,\n  }\n  return\n}",
    );

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message() == "expected `Comma`, found `Identifier`")
        .expect("missing comma diagnostic");

    assert_eq!(diagnostic.code().as_str(), "P0001");
}

#[test]
fn parse_if_condition_allows_struct_literal_inside_call_argument() {
    let source = source(
        "fn main(): null {\n  if isValid(User { permission: permission }) {\n    print(\"yes\")\n  }\n  return\n}",
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
    let condition_node = syntax.node(condition).unwrap();

    assert_eq!(condition_node.kind(), SyntaxNodeKind::CallExpression);
    assert_eq!(
        source.slice(condition_node.span()),
        Some("isValid(User { permission: permission })")
    );

    let arguments = condition_node.children()[1];
    let arguments_node = syntax.node(arguments).unwrap();

    let argument = arguments_node.children()[0];
    let argument_node = syntax.node(argument).unwrap();

    let value = argument_node.children()[0];
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::StructLiteral);
    assert_eq!(
        source.slice(value_node.span()),
        Some("User { permission: permission }")
    );
}

#[test]
fn parse_if_condition_allows_parenthesized_struct_literal() {
    let source = source(
        "fn main(): null {\n  if (User { permission: permission }) {\n    print(\"yes\")\n  }\n  return\n}",
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
    let condition_node = syntax.node(condition).unwrap();

    assert_eq!(condition_node.kind(), SyntaxNodeKind::GroupedExpression);

    let inner = condition_node.children()[0];
    let inner_node = syntax.node(inner).unwrap();

    assert_eq!(inner_node.kind(), SyntaxNodeKind::StructLiteral);
}
