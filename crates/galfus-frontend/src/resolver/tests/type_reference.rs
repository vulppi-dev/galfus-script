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
fn resolve_binds_function_return_named_type() {
    let source = source(
        r#"
        struct User {
            name: [int8],
        }

        fn create(): User {
            return User { name: "Ana" }
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
