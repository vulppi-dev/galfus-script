use super::*;

#[test]
fn parse_anchored_function_declaration() {
    let source = source("fn User::rename(self: User, name: String): User {\n  return self\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    assert_eq!(function_node.kind(), SyntaxNodeKind::FunctionItem);
    assert_eq!(function_node.children().len(), 5);

    let anchor = function_node.children()[0];
    let name = function_node.children()[1];
    let parameters = function_node.children()[2];
    let return_type = function_node.children()[3];
    let body = function_node.children()[4];

    assert_eq!(
        syntax.node(anchor).unwrap().kind(),
        SyntaxNodeKind::FunctionAnchor
    );

    assert_eq!(
        source.slice(syntax.node(anchor).unwrap().span()),
        Some("User")
    );

    let anchor_type = syntax.node(anchor).unwrap().children()[0];

    assert_eq!(
        syntax.node(anchor_type).unwrap().kind(),
        SyntaxNodeKind::TypeName
    );

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("rename")
    );

    assert_eq!(
        syntax.node(parameters).unwrap().kind(),
        SyntaxNodeKind::ParameterList
    );

    assert_eq!(
        syntax.node(return_type).unwrap().kind(),
        SyntaxNodeKind::TypeName
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

    assert_eq!(function_node.kind(), SyntaxNodeKind::FunctionItem);
    assert_eq!(function_node.children().len(), 4);

    assert_eq!(
        source.slice(syntax.node(function_node.children()[0]).unwrap().span()),
        Some("main")
    );

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
fn parse_anchored_generic_function_declaration() {
    let source = source("fn User::convert<T>(self: User): T {\n  return self\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    assert_eq!(function_node.children().len(), 6);

    let anchor = function_node.children()[0];
    let name = function_node.children()[1];
    let generics = function_node.children()[2];
    let parameters = function_node.children()[3];
    let return_type = function_node.children()[4];
    let body = function_node.children()[5];

    assert_eq!(
        syntax.node(anchor).unwrap().kind(),
        SyntaxNodeKind::FunctionAnchor
    );

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("convert")
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
        SyntaxNodeKind::TypeName
    );

    assert_eq!(syntax.node(body).unwrap().kind(), SyntaxNodeKind::Block);
}

#[test]
fn parse_exported_anchored_function_declaration() {
    let source =
        source("export fn User::rename(self: User, name: String): User {\n  return self\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let export = syntax.node(root).unwrap().children()[0];
    let export_node = syntax.node(export).unwrap();

    assert_eq!(export_node.kind(), SyntaxNodeKind::ExportItem);

    let function = export_node.children()[0];
    let function_node = syntax.node(function).unwrap();

    assert_eq!(function_node.kind(), SyntaxNodeKind::FunctionItem);

    let anchor = function_node.children()[0];

    assert_eq!(
        syntax.node(anchor).unwrap().kind(),
        SyntaxNodeKind::FunctionAnchor
    );
}
