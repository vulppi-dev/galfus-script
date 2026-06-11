use super::*;

#[test]
fn parse_copy_expression() {
    let source = source("fn main(): null {\n  const clone = copy value\n  return\n}");

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

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CopyExpression);
    assert_eq!(source.slice(expression_node.span()), Some("copy value"));

    let value = expression_node.first_child().unwrap();

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
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let initializer = syntax.node(statement).unwrap().child(1).unwrap();
    let expression = syntax.node(initializer).unwrap().first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CopyExpression);
    assert_eq!(
        source.slice(expression_node.span()),
        Some("copy user.profile")
    );

    let value = expression_node.first_child().unwrap();

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
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let expression = syntax.node(statement).unwrap().first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    let arguments = expression_node.child(1).unwrap();
    let argument = syntax.node(arguments).unwrap().first_child().unwrap();
    let argument_node = syntax.node(argument).unwrap();

    let value = argument_node.first_child().unwrap();
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
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let statement = syntax.node(body).unwrap().first_child().unwrap();
    let initializer = syntax.node(statement).unwrap().child(1).unwrap();
    let expression = syntax.node(initializer).unwrap().first_child().unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CopyExpression);

    let value = expression_node.first_child().unwrap();

    assert_eq!(
        syntax.node(value).unwrap().kind(),
        SyntaxNodeKind::GroupedExpression
    );

    assert_eq!(
        source.slice(expression_node.span()),
        Some("copy (value + 1)")
    );
}

#[test]
fn parse_weak_struct_field_without_default() {
    let source = source("struct CacheEntry {\n  weak resource: Resource | null,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().first_child().unwrap();
    let struct_node = syntax.node(struct_item).unwrap();

    let fields = struct_node.child(1).unwrap();
    let field = syntax.node(fields).unwrap().first_child().unwrap();
    let field_node = syntax.node(field).unwrap();

    assert_eq!(field_node.kind(), SyntaxNodeKind::WeakStructField);
    assert_eq!(field_node.child_count(), 2);

    let field_type = field_node.child(1).unwrap();

    assert_eq!(
        syntax.node(field_type).unwrap().kind(),
        SyntaxNodeKind::UnionType
    );
}

#[test]
fn parse_weak_struct_field_non_nullable_is_syntax_valid() {
    let source = source("struct CacheEntry {\n  weak resource: Resource,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().first_child().unwrap();
    let struct_node = syntax.node(struct_item).unwrap();

    let fields = struct_node.child(1).unwrap();
    let field = syntax.node(fields).unwrap().first_child().unwrap();
    let field_node = syntax.node(field).unwrap();

    assert_eq!(field_node.kind(), SyntaxNodeKind::WeakStructField);
    assert_eq!(
        source.slice(field_node.span()),
        Some("weak resource: Resource")
    );
}

#[test]
fn parse_regular_struct_field_still_uses_struct_field() {
    let source = source("struct User {\n  name: String = \"Anonymous\",\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().first_child().unwrap();
    let struct_node = syntax.node(struct_item).unwrap();

    let fields = struct_node.child(1).unwrap();
    let field = syntax.node(fields).unwrap().first_child().unwrap();
    let field_node = syntax.node(field).unwrap();

    assert_eq!(field_node.kind(), SyntaxNodeKind::StructField);
    assert_eq!(field_node.child_count(), 3);

    let name = field_node.first_child().unwrap();
    let field_type = field_node.child(1).unwrap();
    let default = field_node.child(2).unwrap();

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("name")
    );

    assert_eq!(
        syntax.node(field_type).unwrap().kind(),
        SyntaxNodeKind::NamedType
    );

    assert_eq!(
        syntax.node(default).unwrap().kind(),
        SyntaxNodeKind::StructFieldDefault
    );
}
