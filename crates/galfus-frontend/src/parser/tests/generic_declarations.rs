use super::*;

#[test]
fn parse_generic_struct_declaration() {
    let source = source("struct Box<T> {\n  value: T,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    assert_eq!(struct_node.kind(), SyntaxNodeKind::StructItem);
    assert_eq!(struct_node.children().len(), 3);

    let name = struct_node.children()[0];
    let generics = struct_node.children()[1];
    let fields = struct_node.children()[2];

    assert_eq!(source.slice(syntax.node(name).unwrap().span()), Some("Box"));

    let generics_node = syntax.node(generics).unwrap();

    assert_eq!(generics_node.kind(), SyntaxNodeKind::GenericParameterList);

    assert_eq!(generics_node.children().len(), 1);
    assert_eq!(source.slice(generics_node.span()), Some("<T>"));

    let generic = generics_node.children()[0];

    assert_eq!(
        syntax.node(generic).unwrap().kind(),
        SyntaxNodeKind::GenericParameter
    );

    assert_eq!(
        source.slice(syntax.node(generic).unwrap().span()),
        Some("T")
    );

    assert_eq!(
        syntax.node(fields).unwrap().kind(),
        SyntaxNodeKind::StructFieldList
    );
}

#[test]
fn parse_generic_struct_declaration_with_multiple_parameters() {
    let source = source("struct Pair<T, U> {\n  first: T,\n  second: U,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    let generics = struct_node.children()[1];
    let generics_node = syntax.node(generics).unwrap();

    assert_eq!(generics_node.kind(), SyntaxNodeKind::GenericParameterList);
    assert_eq!(generics_node.children().len(), 2);
    assert_eq!(source.slice(generics_node.span()), Some("<T, U>"));
}

#[test]
fn parse_regular_struct_declaration_shape_is_unchanged() {
    let source = source("struct User {\n  name: String,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    assert_eq!(struct_node.children().len(), 2);

    assert_eq!(
        syntax.node(struct_node.children()[1]).unwrap().kind(),
        SyntaxNodeKind::StructFieldList
    );
}

#[test]
fn parse_generic_function_declaration() {
    let source = source("fn identity<T>(value: T): T {\n  return value\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    assert_eq!(function_node.kind(), SyntaxNodeKind::FunctionItem);
    assert_eq!(function_node.children().len(), 5);

    let name = function_node.children()[0];
    let generics = function_node.children()[1];
    let parameters = function_node.children()[2];
    let return_type = function_node.children()[3];
    let body = function_node.children()[4];

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("identity")
    );

    assert_eq!(
        syntax.node(generics).unwrap().kind(),
        SyntaxNodeKind::GenericParameterList
    );

    assert_eq!(
        source.slice(syntax.node(generics).unwrap().span()),
        Some("<T>")
    );

    assert_eq!(
        syntax.node(parameters).unwrap().kind(),
        SyntaxNodeKind::ParameterList
    );

    assert_eq!(
        syntax.node(return_type).unwrap().kind(),
        SyntaxNodeKind::NamedType
    );

    assert_eq!(syntax.node(body).unwrap().kind(), SyntaxNodeKind::Block);
}

#[test]
fn parse_regular_function_declaration_shape_is_unchanged() {
    let source = source("fn main(): null {\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    assert_eq!(function_node.children().len(), 4);

    assert_eq!(
        syntax.node(function_node.children()[1]).unwrap().kind(),
        SyntaxNodeKind::ParameterList
    );

    assert_eq!(
        syntax.node(function_node.children()[2]).unwrap().kind(),
        SyntaxNodeKind::TypeNull
    );

    assert_eq!(
        syntax.node(function_node.children()[3]).unwrap().kind(),
        SyntaxNodeKind::Block
    );
}

#[test]
fn parse_generic_parameter_list_with_trailing_comma() {
    let source = source("struct Pair<T, U,> {\n  first: T,\n  second: U,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let struct_item = syntax.node(root).unwrap().children()[0];
    let struct_node = syntax.node(struct_item).unwrap();

    let generics = struct_node.children()[1];
    let generics_node = syntax.node(generics).unwrap();

    assert_eq!(generics_node.children().len(), 2);
}

#[test]
fn parse_generic_parameter_list_with_newlines() {
    let source = source("struct Pair<\n  T,\n  U,\n> {\n  first: T,\n  second: U,\n}");

    let result = parse(&source);

    assert!(!result.has_errors());
}
