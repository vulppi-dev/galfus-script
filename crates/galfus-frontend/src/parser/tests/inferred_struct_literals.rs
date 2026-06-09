use super::*;

#[test]
fn parse_struct_field_default() {
    let source = source("struct Person {\n  name: String = \"Anonymous\",\n  age: uint32 = 0,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    assert_eq!(struct_node.kind(), SyntaxNodeKind::StructItem);

    let fields = struct_node.children()[1];
    let fields_node = syntax.node(fields).unwrap();

    assert_eq!(fields_node.children().len(), 2);

    let first_field = fields_node.children()[0];
    let first_field_node = syntax.node(first_field).unwrap();

    assert_eq!(first_field_node.kind(), SyntaxNodeKind::StructField);
    assert_eq!(
        source.slice(first_field_node.span()),
        Some("name: String = \"Anonymous\"")
    );

    assert_eq!(first_field_node.children().len(), 3);

    let default = first_field_node.children()[2];
    let default_node = syntax.node(default).unwrap();

    assert_eq!(default_node.kind(), SyntaxNodeKind::StructFieldDefault);
    assert_eq!(source.slice(default_node.span()), Some("= \"Anonymous\""));
}

#[test]
fn parse_struct_field_default_with_union_null() {
    let source = source("struct Person {\n  email: String | null = null,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    let fields = struct_node.children()[1];
    let field = syntax.node(fields).unwrap().children()[0];
    let field_node = syntax.node(field).unwrap();

    let field_type = field_node.children()[1];
    let default = field_node.children()[2];

    assert_eq!(
        syntax.node(field_type).unwrap().kind(),
        SyntaxNodeKind::UnionType
    );

    assert_eq!(
        syntax.node(default).unwrap().kind(),
        SyntaxNodeKind::StructFieldDefault
    );

    assert_eq!(
        source.slice(field_node.span()),
        Some("email: String | null = null")
    );
}

#[test]
fn parse_inferred_struct_literal_as_call_argument() {
    let source = source(
        "fn main(): null {\n  createPerson(struct { email: \"user@gmail.com\" })\n  return\n}",
    );

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

    assert_eq!(argument_node.kind(), SyntaxNodeKind::Argument);

    let value = argument_node.children()[0];
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::InferredStructLiteral);
    assert_eq!(
        source.slice(value_node.span()),
        Some("struct { email: \"user@gmail.com\" }")
    );
}

#[test]
fn parse_inferred_struct_literal_with_multiple_fields() {
    let source = source(
        "fn main(): null {\n  createPerson(struct {\n    name: \"Ana\",\n    email: \"ana@gmail.com\",\n  })\n  return\n}",
    );

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

    let arguments = expression_node.children()[1];
    let argument = syntax.node(arguments).unwrap().children()[0];
    let value = syntax.node(argument).unwrap().children()[0];
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::InferredStructLiteral);

    let fields = value_node.children()[0];
    let fields_node = syntax.node(fields).unwrap();

    assert_eq!(fields_node.kind(), SyntaxNodeKind::StructLiteralFieldList);
    assert_eq!(fields_node.children().len(), 2);
}

#[test]
fn parse_inferred_struct_literal_with_shorthand_field() {
    let source = source("fn main(): null {\n  createPerson(struct { email })\n  return\n}");

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

    let arguments = expression_node.children()[1];
    let argument = syntax.node(arguments).unwrap().children()[0];
    let value = syntax.node(argument).unwrap().children()[0];
    let value_node = syntax.node(value).unwrap();

    let fields = value_node.children()[0];
    let field = syntax.node(fields).unwrap().children()[0];
    let field_node = syntax.node(field).unwrap();

    assert_eq!(
        field_node.kind(),
        SyntaxNodeKind::StructLiteralFieldShorthand
    );

    assert_eq!(source.slice(field_node.span()), Some("email"));
}

#[test]
fn parse_inferred_struct_literal_in_return() {
    let source =
        source("fn makePerson(): Person {\n  return struct { email: \"user@gmail.com\" }\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let return_statement = syntax.node(body).unwrap().children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    let value = return_node.children()[0];
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::InferredStructLiteral);
    assert_eq!(
        source.slice(value_node.span()),
        Some("struct { email: \"user@gmail.com\" }")
    );
}
