use super::*;

#[test]
fn parse_type_alias_function_type_no_parameters() {
    let source = source("type Callback = fn (): null");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let alias = syntax.node(root).unwrap().children()[0];
    let alias_node = syntax.node(alias).unwrap();

    let function_type = alias_node.children()[1];
    let function_type_node = syntax.node(function_type).unwrap();

    assert_eq!(function_type_node.kind(), SyntaxNodeKind::FunctionType);
    assert_eq!(source.slice(function_type_node.span()), Some("fn (): null"));

    let parameters = function_type_node.children()[0];
    let return_type = function_type_node.children()[1];

    assert_eq!(
        syntax.node(parameters).unwrap().kind(),
        SyntaxNodeKind::FunctionTypeParameterList
    );

    assert_eq!(syntax.node(parameters).unwrap().children().len(), 0);

    assert_eq!(
        syntax.node(return_type).unwrap().kind(),
        SyntaxNodeKind::TypeNull
    );
}

#[test]
fn parse_type_alias_function_type_with_parameters() {
    let source = source("type Predicate = fn (String, int32): bool");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let alias = syntax.node(root).unwrap().children()[0];
    let alias_node = syntax.node(alias).unwrap();

    let function_type = alias_node.children()[1];
    let function_type_node = syntax.node(function_type).unwrap();

    assert_eq!(function_type_node.kind(), SyntaxNodeKind::FunctionType);
    assert_eq!(
        source.slice(function_type_node.span()),
        Some("fn (String, int32): bool")
    );

    let parameters = function_type_node.children()[0];
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.children().len(), 2);

    let first = parameters_node.children()[0];
    let second = parameters_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(first).unwrap().span()),
        Some("String")
    );

    assert_eq!(
        source.slice(syntax.node(second).unwrap().span()),
        Some("int32")
    );
}

#[test]
fn parse_struct_field_function_type() {
    let source = source("struct Button {\n  onClick: fn (): null,\n}");

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

    assert_eq!(
        syntax.node(field_type).unwrap().kind(),
        SyntaxNodeKind::FunctionType
    );

    assert_eq!(
        source.slice(syntax.node(field_type).unwrap().span()),
        Some("fn (): null")
    );
}

#[test]
fn parse_var_type_annotation_function_type() {
    let source =
        source("fn main(): null {\n  var callback: fn (): null = handleClick\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let statement_node = syntax.node(statement).unwrap();

    let type_annotation = statement_node.children()[1];
    let type_annotation_node = syntax.node(type_annotation).unwrap();

    assert_eq!(type_annotation_node.kind(), SyntaxNodeKind::TypeAnnotation);

    let function_type = type_annotation_node.children()[0];

    assert_eq!(
        syntax.node(function_type).unwrap().kind(),
        SyntaxNodeKind::FunctionType
    );
}

#[test]
fn parse_generic_function_type() {
    let source = source("type Transform<T, U> = fn (T): U");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let alias = syntax.node(root).unwrap().children()[0];
    let alias_node = syntax.node(alias).unwrap();

    let function_type = alias_node.children()[2];
    let function_type_node = syntax.node(function_type).unwrap();

    assert_eq!(function_type_node.kind(), SyntaxNodeKind::FunctionType);
    assert_eq!(source.slice(function_type_node.span()), Some("fn (T): U"));

    let parameters = function_type_node.children()[0];
    let parameter = syntax.node(parameters).unwrap().children()[0];

    assert_eq!(
        source.slice(syntax.node(parameter).unwrap().span()),
        Some("T")
    );

    let return_type = function_type_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(return_type).unwrap().span()),
        Some("U")
    );
}

#[test]
fn parse_function_type_as_generic_argument() {
    let source = source("type Listener = Box<fn (String): null>");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let alias = syntax.node(root).unwrap().children()[0];
    let alias_node = syntax.node(alias).unwrap();

    let generic_type = alias_node.children()[1];
    let generic_type_node = syntax.node(generic_type).unwrap();

    assert_eq!(generic_type_node.kind(), SyntaxNodeKind::GenericType);

    let args = generic_type_node.children()[1];
    let first_arg = syntax.node(args).unwrap().children()[0];

    assert_eq!(
        syntax.node(first_arg).unwrap().kind(),
        SyntaxNodeKind::FunctionType
    );
}
