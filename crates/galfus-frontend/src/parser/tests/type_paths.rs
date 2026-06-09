use super::*;

#[test]
fn parse_type_alias_type_path() {
    let source = source("type TextureAlias = game::Texture");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let alias = syntax.node(root).unwrap().children()[0];
    let alias_node = syntax.node(alias).unwrap();

    let alias_type = alias_node.children()[1];
    let alias_type_node = syntax.node(alias_type).unwrap();

    assert_eq!(alias_type_node.kind(), SyntaxNodeKind::TypePath);
    assert_eq!(source.slice(alias_type_node.span()), Some("game::Texture"));

    assert_eq!(alias_type_node.children().len(), 2);
}

#[test]
fn parse_type_alias_generic_type_path() {
    let source = source("type UserResult = std::Result<User, Error>");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let alias = syntax.node(root).unwrap().children()[0];
    let alias_node = syntax.node(alias).unwrap();

    let alias_type = alias_node.children()[1];
    let alias_type_node = syntax.node(alias_type).unwrap();

    assert_eq!(alias_type_node.kind(), SyntaxNodeKind::GenericType);
    assert_eq!(
        source.slice(alias_type_node.span()),
        Some("std::Result<User, Error>")
    );

    let base = alias_type_node.children()[0];

    assert_eq!(syntax.node(base).unwrap().kind(), SyntaxNodeKind::TypePath);

    assert_eq!(
        source.slice(syntax.node(base).unwrap().span()),
        Some("std::Result")
    );
}

#[test]
fn parse_struct_field_type_path() {
    let source = source("struct Asset {\n  texture: game::Texture,\n}");

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
    let field_type_node = syntax.node(field_type).unwrap();

    assert_eq!(field_type_node.kind(), SyntaxNodeKind::TypePath);
    assert_eq!(source.slice(field_type_node.span()), Some("game::Texture"));
}

#[test]
fn parse_simple_anchored_function_still_works_with_type_path() {
    let source = source("fn User::rename(self: User): User {\n  return self\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let anchor = function_node.children()[0];
    let name = function_node.children()[1];

    assert_eq!(
        syntax.node(anchor).unwrap().kind(),
        SyntaxNodeKind::FunctionAnchor
    );

    assert_eq!(
        source.slice(syntax.node(anchor).unwrap().span()),
        Some("User")
    );

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("rename")
    );
}

#[test]
fn parse_type_path_anchored_function() {
    let source = source("fn game::Texture::load(self: game::Texture): null {\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let anchor = function_node.children()[0];
    let name = function_node.children()[1];

    let anchor_node = syntax.node(anchor).unwrap();

    assert_eq!(anchor_node.kind(), SyntaxNodeKind::FunctionAnchor);
    assert_eq!(source.slice(anchor_node.span()), Some("game::Texture"));

    let anchor_type = anchor_node.children()[0];

    assert_eq!(
        syntax.node(anchor_type).unwrap().kind(),
        SyntaxNodeKind::TypePath
    );

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("load")
    );
}

#[test]
fn parse_generic_type_path_anchored_function() {
    let source = source("fn std::Box<T>::unwrap(self: std::Box<T>): T {\n  return self.value\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let anchor = function_node.children()[0];
    let anchor_node = syntax.node(anchor).unwrap();

    assert_eq!(anchor_node.kind(), SyntaxNodeKind::FunctionAnchor);
    assert_eq!(source.slice(anchor_node.span()), Some("std::Box<T>"));

    let anchor_type = anchor_node.children()[0];

    assert_eq!(
        syntax.node(anchor_type).unwrap().kind(),
        SyntaxNodeKind::GenericType
    );

    let generic_base = syntax.node(anchor_type).unwrap().children()[0];

    assert_eq!(
        syntax.node(generic_base).unwrap().kind(),
        SyntaxNodeKind::TypePath
    );
}
