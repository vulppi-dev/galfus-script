use super::*;
use crate::SyntaxLayer;
use galfus_core::NodeId;

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "main.gfs".to_string(), text.to_string())
}

fn find_named_type_by_text(
    syntax: &SyntaxLayer,
    source: &SourceFile,
    node: NodeId,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = syntax.node(node)?;

    if syntax_node.kind() == SyntaxNodeKind::NamedType {
        if let Some(identifier) = syntax.first_child_of_kind(node, SyntaxNodeKind::Identifier) {
            if source.slice(syntax.node(identifier)?.span()) == Some(text) {
                return Some(node);
            }
        }
    }

    for child in syntax_node.children() {
        if let Some(found) = find_named_type_by_text(syntax, source, *child, text) {
            return Some(found);
        }
    }

    None
}

fn collect_named_types_by_text(
    syntax: &SyntaxLayer,
    source: &SourceFile,
    node: NodeId,
    text: &str,
    found: &mut Vec<NodeId>,
) {
    let Some(syntax_node) = syntax.node(node) else {
        return;
    };

    if syntax_node.kind() == SyntaxNodeKind::NamedType {
        if let Some(identifier) = syntax.first_child_of_kind(node, SyntaxNodeKind::Identifier) {
            if source.slice(syntax.node(identifier).unwrap().span()) == Some(text) {
                found.push(node);
            }
        }
    }

    for child in syntax_node.children() {
        collect_named_types_by_text(syntax, source, *child, text, found);
    }
}

fn find_path_type_by_text(
    syntax: &SyntaxLayer,
    source: &SourceFile,
    node: NodeId,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = syntax.node(node)?;

    if syntax_node.kind() == SyntaxNodeKind::Path && source.slice(syntax_node.span()) == Some(text)
    {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_path_type_by_text(syntax, source, *child, text) {
            return Some(found);
        }
    }

    None
}

fn find_function_anchor_by_text(
    syntax: &SyntaxLayer,
    source: &SourceFile,
    node: NodeId,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = syntax.node(node)?;

    if syntax_node.kind() == SyntaxNodeKind::FunctionAnchor
        && source.slice(syntax_node.span()) == Some(text)
    {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_function_anchor_by_text(syntax, source, *child, text) {
            return Some(found);
        }
    }

    None
}

#[test]
fn resolve_binds_function_parameter_named_type() {
    let source = source(
        r#"
        struct User {
            name: [int8],
        }

        fn greet(user: User): null {
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let named_type = find_named_type_by_text(syntax, &source, root, "User").unwrap();

    let symbol = resolution.type_reference_symbol(named_type).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "User");
    assert_eq!(symbol.kind(), SymbolKind::Struct);
}

#[test]
fn resolve_accepts_struct_function_anchor() {
    let source = source(
        r#"
        struct User {
            name: [int8],
        }

        fn User::rename(self: User, name: [int8]): User {
            return self
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();
    let anchor = find_function_anchor_by_text(syntax, &source, root, "User").unwrap();
    let anchor_type = syntax.first_child(anchor).unwrap();

    let symbol = resolution
        .symbol(resolution.type_reference_symbol(anchor_type).unwrap())
        .unwrap();

    assert_eq!(symbol.name(), "User");
    assert_eq!(symbol.kind(), SymbolKind::Struct);
}

#[test]
fn resolve_reports_non_struct_function_anchor() {
    let source = source(
        r#"
        choice Result<V, F> {
            Ok(V),
            Err(F),
        }

        fn Result::map(self: Result<int32, [int8]>): Result<int32, [int8]> {
            return self
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();

    assert!(graph.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == "R0004"
            && diagnostic
                .message()
                .contains("function anchor `Result` must be a struct")
    }));
}

#[test]
fn resolve_binds_function_return_named_type() {
    let source = source(
        r#"
        struct User {
            name: [int8],
        }

        fn create(): User {
            return new(User) { name: "Ana" }
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let named_type = find_named_type_by_text(syntax, &source, root, "User").unwrap();

    let symbol = resolution.type_reference_symbol(named_type).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "User");
    assert_eq!(symbol.kind(), SymbolKind::Struct);
}

#[test]
fn resolve_binds_arrow_function_signature_named_types() {
    let source = source(
        r#"
        struct User {
            name: [int8],
        }

        fn main(): null {
            const identity = (user: User): User => user
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();
    let mut user_types = Vec::new();
    collect_named_types_by_text(syntax, &source, root, "User", &mut user_types);

    assert_eq!(user_types.len(), 2);

    for named_type in user_types {
        let symbol = resolution
            .symbol(resolution.type_reference_symbol(named_type).unwrap())
            .unwrap();

        assert_eq!(symbol.name(), "User");
        assert_eq!(symbol.kind(), SymbolKind::Struct);
    }
}

#[test]
fn resolve_binds_struct_field_named_type() {
    let source = source(
        r#"
        struct Profile {
            bio: [int8],
        }

        struct User {
            profile: Profile,
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let named_type = find_named_type_by_text(syntax, &source, root, "Profile").unwrap();

    let symbol = resolution.type_reference_symbol(named_type).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "Profile");
    assert_eq!(symbol.kind(), SymbolKind::Struct);
}

#[test]
fn resolve_binds_builtin_named_types() {
    let source = source(
        r#"
        fn main(value: int32): [int8] {
            return "ok"
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let int_type = find_named_type_by_text(syntax, &source, root, "int32").unwrap();
    let int8_type = find_named_type_by_text(syntax, &source, root, "int8").unwrap();

    let int_symbol = resolution
        .symbol(resolution.type_reference_symbol(int_type).unwrap())
        .unwrap();

    let int8_symbol = resolution
        .symbol(resolution.type_reference_symbol(int8_type).unwrap())
        .unwrap();

    assert_eq!(int_symbol.kind(), SymbolKind::BuiltinType);
    assert_eq!(int_symbol.name(), "int32");

    assert_eq!(int8_symbol.kind(), SymbolKind::BuiltinType);
    assert_eq!(int8_symbol.name(), "int8");
}

#[test]
fn resolve_binds_import_namespace_type_path_root() {
    let source = source(
        r#"
        import user from "./user"

        type LocalUser = user::User
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let path_type = find_path_type_by_text(syntax, &source, root, "user::User").unwrap();

    let symbol = resolution.type_reference_symbol(path_type).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "user");
    assert_eq!(symbol.kind(), SymbolKind::ImportNamespace);
}

#[test]
fn resolve_binds_local_type_path_root() {
    let source = source(
        r#"
        struct User {
            Id: int32,
        }

        type LocalUserId = User::Id
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let path_type = find_path_type_by_text(syntax, &source, root, "User::Id").unwrap();

    let symbol = resolution.type_reference_symbol(path_type).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "User");
    assert_eq!(symbol.kind(), SymbolKind::Struct);
}

#[test]
fn resolve_binds_local_type_path_member() {
    let source = source(
        r#"
        struct User {
            Id: int32,
        }

        type LocalUserId = User::Id
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let path_type = find_path_type_by_text(syntax, &source, root, "User::Id").unwrap();

    let root_symbol = resolution
        .symbol(resolution.type_reference_symbol(path_type).unwrap())
        .unwrap();
    let member_symbol = resolution
        .symbol(resolution.type_path_reference_symbol(path_type).unwrap())
        .unwrap();

    assert_eq!(root_symbol.name(), "User");
    assert_eq!(root_symbol.kind(), SymbolKind::Struct);

    assert_eq!(member_symbol.name(), "Id");
    assert_eq!(member_symbol.kind(), SymbolKind::StructField);
}

#[test]
fn resolve_binds_constraint_field_type_path_member() {
    let source = source(
        r#"
        constraint Entity {
            Id: int64,
        }

        type EntityId = Entity::Id
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let path_type = find_path_type_by_text(syntax, &source, root, "Entity::Id").unwrap();

    let root_symbol = resolution
        .symbol(resolution.type_reference_symbol(path_type).unwrap())
        .unwrap();
    let member_symbol = resolution
        .symbol(resolution.type_path_reference_symbol(path_type).unwrap())
        .unwrap();

    assert_eq!(root_symbol.name(), "Entity");
    assert_eq!(root_symbol.kind(), SymbolKind::Constraint);

    assert_eq!(member_symbol.name(), "Id");
    assert_eq!(member_symbol.kind(), SymbolKind::ConstraintField);
}

#[test]
fn resolve_reports_unknown_local_type_path_member() {
    let source = source(
        r#"
        struct User {
            Id: int32,
        }

        type LocalUserName = User::Name
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let path_type = find_path_type_by_text(syntax, &source, root, "User::Name").unwrap();

    assert!(resolution.type_reference_symbol(path_type).is_some());
    assert!(resolution.type_path_reference_symbol(path_type).is_none());

    assert!(graph.diagnostics().iter().any(|diagnostic| {
        diagnostic
            .message()
            .contains("unresolved type path member `Name`")
    }));
}

#[test]
fn resolve_reports_unknown_type_path_root() {
    let source = source(
        r#"
        type LocalUser = missing::User
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let path_type = find_path_type_by_text(syntax, &source, root, "missing::User").unwrap();

    assert!(resolution.type_reference_symbol(path_type).is_none());
}

#[test]
fn resolve_reports_value_symbol_as_invalid_type_path_root() {
    let source = source(
        r#"
        const user = 0

        type LocalUser = user::User
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let path_type = find_path_type_by_text(syntax, &source, root, "user::User").unwrap();

    assert!(resolution.type_reference_symbol(path_type).is_none());
}

#[test]
fn resolve_reports_unknown_named_type() {
    let source = source(
        r#"
        fn main(value: MissingType): null {
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let named_type = find_named_type_by_text(syntax, &source, root, "MissingType").unwrap();

    assert!(resolution.type_reference_symbol(named_type).is_none());
}

#[test]
fn resolve_reports_excluded_primitive_names_as_unknown_types() {
    let source = source(
        r#"
        fn main(text: String, ch: char): null {
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());
}

#[test]
fn resolve_binds_choice_generic_parameter_type_references() {
    let source = source(
        r#"
        choice Result<V, F> {
            Ok(V),
            Err(F),
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let v_type = find_named_type_by_text(syntax, &source, root, "V").unwrap();
    let f_type = find_named_type_by_text(syntax, &source, root, "F").unwrap();

    let v_symbol = resolution
        .symbol(resolution.type_reference_symbol(v_type).unwrap())
        .unwrap();

    let f_symbol = resolution
        .symbol(resolution.type_reference_symbol(f_type).unwrap())
        .unwrap();

    assert_eq!(v_symbol.kind(), SymbolKind::GenericParameter);
    assert_eq!(v_symbol.name(), "V");

    assert_eq!(f_symbol.kind(), SymbolKind::GenericParameter);
    assert_eq!(f_symbol.name(), "F");
}

#[test]
fn resolve_binds_constraint_generic_parameter_type_references() {
    let source = source(
        r#"
        constraint Stringable<T> {
            fn toString(self: T): [int8]
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let t_type = find_named_type_by_text(syntax, &source, root, "T").unwrap();

    let t_symbol = resolution
        .symbol(resolution.type_reference_symbol(t_type).unwrap())
        .unwrap();

    assert_eq!(t_symbol.kind(), SymbolKind::GenericParameter);
    assert_eq!(t_symbol.name(), "T");
}
