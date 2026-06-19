use super::*;

use crate::{TypeKind, parse, resolve};

fn source(text: &str) -> SourceFile {
    SourceFile::new(
        galfus_core::SourceId::new(0),
        "test.gfs".to_string(),
        text.to_string(),
    )
}

fn lower_source(text: &str) -> TypeLoweringResult {
    let source = source(text);
    let parse_result = parse(&source);

    assert!(
        !parse_result.has_errors(),
        "{:?}",
        parse_result.diagnostics()
    );

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(
        !resolve_result.has_errors(),
        "{:?}",
        resolve_result.diagnostics()
    );

    lower_types(&source, resolve_result.graph())
}

fn find_node_by_kind_and_text(
    source: &SourceFile,
    graph: &ModuleGraph,
    kind: SyntaxNodeKind,
    text: &str,
) -> Option<NodeId> {
    let root = graph.syntax().root()?;
    find_node_by_kind_and_text_from(source, graph, root, kind, text)
}

fn find_node_by_kind_and_text_from(
    source: &SourceFile,
    graph: &ModuleGraph,
    node: NodeId,
    kind: SyntaxNodeKind,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = graph.syntax().node(node)?;

    if syntax_node.kind() == kind && source.slice(syntax_node.span()) == Some(text) {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_node_by_kind_and_text_from(source, graph, *child, kind, text) {
            return Some(found);
        }
    }

    None
}

#[test]
fn lower_binds_primitive_named_type() {
    let source = source(
        r#"
fn main(value: int32): null {
  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let result = lower_types(&source, resolve_result.graph());
    let graph = resolve_result.graph();

    let int32_node =
        find_node_by_kind_and_text(&source, graph, SyntaxNodeKind::NamedType, "int32").unwrap();

    let ty = result.layer().node_type(int32_node).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn lower_binds_array_type() {
    let source = source(
        r#"
fn main(values: [int32]): null {
  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let result = lower_types(&source, resolve_result.graph());
    let graph = resolve_result.graph();

    let array_node =
        find_node_by_kind_and_text(&source, graph, SyntaxNodeKind::ArrayType, "[int32]").unwrap();

    let ty = result.layer().node_type(array_node).unwrap();

    match result.layer().table().kind(ty) {
        Some(TypeKind::Array { element }) => {
            assert_eq!(
                result.layer().table().kind(*element),
                Some(&TypeKind::Primitive(PrimitiveType::Int32))
            );
        }
        other => panic!("expected array type, got {other:?}"),
    }
}

#[test]
fn lower_binds_fixed_array_type() {
    let source = source(
        r#"
fn main(values: [int32; 4]): null {
  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let result = lower_types(&source, resolve_result.graph());
    let graph = resolve_result.graph();

    let array_node =
        find_node_by_kind_and_text(&source, graph, SyntaxNodeKind::FixedArrayType, "[int32; 4]")
            .unwrap();

    let ty = result.layer().node_type(array_node).unwrap();

    match result.layer().table().kind(ty) {
        Some(TypeKind::FixedArray { size, .. }) => {
            assert_eq!(*size, ArraySize::Known(4));
        }
        other => panic!("expected fixed array type, got {other:?}"),
    }
}

#[test]
fn lower_normalizes_union_type() {
    let source = source(
        r#"
fn main(value: int32 | null | int32): null {
  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let result = lower_types(&source, resolve_result.graph());
    let graph = resolve_result.graph();

    let union_node = find_node_by_kind_and_text(
        &source,
        graph,
        SyntaxNodeKind::UnionType,
        "int32 | null | int32",
    )
    .unwrap();

    let ty = result.layer().node_type(union_node).unwrap();

    match result.layer().table().kind(ty) {
        Some(TypeKind::Union { members }) => {
            assert_eq!(members.len(), 2);
        }
        other => panic!("expected union type, got {other:?}"),
    }
}

#[test]
fn lower_binds_named_struct_type() {
    let source = source(
        r#"
struct User {
  id: int64,
}

fn main(value: User): null {
  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let result = lower_types(&source, resolve_result.graph());
    let graph = resolve_result.graph();

    let user_node =
        find_node_by_kind_and_text(&source, graph, SyntaxNodeKind::NamedType, "User").unwrap();

    let ty = result.layer().node_type(user_node).unwrap();

    assert!(matches!(
        result.layer().table().kind(ty),
        Some(TypeKind::Named { .. })
    ));
}

#[test]
fn lower_binds_external_path_type() {
    let source = source(
        r#"
import user from "./user"

fn main(value: user::User): null {
  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let result = lower_types(&source, resolve_result.graph());
    let graph = resolve_result.graph();

    let path_node =
        find_node_by_kind_and_text(&source, graph, SyntaxNodeKind::Path, "user::User").unwrap();

    let ty = result.layer().node_type(path_node).unwrap();

    match result.layer().table().kind(ty) {
        Some(TypeKind::Path { segments, .. }) => {
            assert_eq!(segments, &vec!["User".to_string()]);
        }
        other => panic!("expected path type, got {other:?}"),
    }
}

#[test]
fn lower_binds_generic_instance_type() {
    let source = source(
        r#"
choice Result<V, E> {
  Ok(V),
  Err(E),
}

struct Error {
  code: int32,
}

fn main(value: Result<int32, Error>): null {
  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let result = lower_types(&source, resolve_result.graph());
    let graph = resolve_result.graph();

    let generic_node = find_node_by_kind_and_text(
        &source,
        graph,
        SyntaxNodeKind::GenericType,
        "Result<int32, Error>",
    )
    .unwrap();

    let ty = result.layer().node_type(generic_node).unwrap();

    assert!(matches!(
        result.layer().table().kind(ty),
        Some(TypeKind::GenericInstance { .. })
    ));
}
