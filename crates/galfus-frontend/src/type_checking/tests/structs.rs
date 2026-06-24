use super::*;

#[test]
fn check_accepts_struct_literal() {
    let (_source, _graph, result) = check_source(
        r#"
struct User {
  id: int32,
  name: [uint8],
}

var user: User = User {
  id: 1,
  name: "Ana",
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_binds_struct_literal_type() {
    let (_, graph, result) = check_source(
        r#"
struct User {
  id: int32,
  name: [uint8],
}

var user: User = User {
  id: 1,
  name: "Ana",
}
"#,
    );

    let literal = find_node_by_kind(&graph, SyntaxNodeKind::StructLiteral).unwrap();

    let user_symbol = symbol_by_name_and_kind(&graph, "User", SymbolKind::Struct);
    let ty = result.layer().node_type(literal).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Named {
            symbol: user_symbol
        })
    );
}

#[test]
fn check_reports_unknown_struct_field() {
    let source = source(
        r#"
struct User {
  id: int32,
}

var user: User = User {
  id: 1,
  name: "Ana",
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::UnknownStructField.as_code()
            && diagnostic.message().contains("has no field `name`")
    }));
}

#[test]
fn check_reports_duplicate_struct_field() {
    let source = source(
        r#"
struct User {
  id: int32,
}

var user: User = User {
  id: 1,
  id: 2,
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::DuplicateStructField.as_code()
            && diagnostic.message().contains("duplicate field `id`")
    }));
}

#[test]
fn check_reports_missing_required_struct_field() {
    let source = source(
        r#"
struct User {
  id: int32,
  name: [uint8],
}

var user: User = User {
  id: 1,
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::MissingStructField.as_code()
            && diagnostic
                .message()
                .contains("missing required field `name`")
    }));
}

#[test]
fn check_accepts_missing_default_struct_field() {
    let (_source, _graph, result) = check_source(
        r#"
struct User {
  id: int32,
  age: int32 = 0,
}

var user: User = User {
  id: 1,
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_assignment_to_mutable_struct_field() {
    let (_source, _graph, result) = check_source(
        r#"
struct User {
  id: int32,
}

var user: User = User { id: 1 }

fn update(): null {
  user.id = 2
  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_assignment_to_const_struct_field() {
    let source = source(
        r#"
struct User {
  const id: int32,
}

var user: User = User { id: 1 }

fn update(): null {
  user.id = 2
  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::AssignmentToImmutable.as_code()
            && diagnostic
                .message()
                .contains("cannot assign to immutable binding `id`")
    }));
}

#[test]
fn check_reports_struct_field_type_mismatch() {
    let source = source(
        r#"
struct User {
  id: int32,
}

var user: User = User {
  id: true,
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::TypeMismatch.as_code()
            && diagnostic
                .message()
                .contains("expected `int32`, got `bool`")
    }));
}

#[test]
fn check_accepts_struct_literal_shorthand() {
    let (_source, _graph, result) = check_source(
        r#"
struct User {
  id: int32,
  name: [uint8],
}

var id: int32 = 1
var name: [uint8] = "Ana"

var user: User = User {
  id,
  name,
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_collects_weak_field_ownership_metadata() {
    let (_source, graph, result) = check_source(
        r#"
struct Node {
  value: int32,
  weak parent: Node | null,
}
"#,
    );

    let weak_fields = result.ownership_metadata().weak_fields();

    assert_eq!(weak_fields.len(), 1);

    let metadata = weak_fields[0];
    let node_symbol = symbol_by_name_and_kind(&graph, "Node", SymbolKind::Struct);
    let parent_symbol = symbol_by_name_and_kind(&graph, "parent", SymbolKind::StructField);

    assert_eq!(metadata.struct_symbol(), node_symbol);
    assert_eq!(metadata.field_symbol(), parent_symbol);

    assert!(matches!(
        result.layer().table().kind(metadata.field_type()),
        Some(TypeKind::Union { .. })
    ));
}

#[test]
fn check_reports_non_nullable_weak_field_type() {
    let source = source(
        r#"
struct Node {
  weak parent: Node,
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidWeakFieldType.as_code()
            && diagnostic
                .message()
                .contains("weak field type must be nullable")
    }));
    assert_eq!(result.ownership_metadata().weak_fields().len(), 1);
}

#[test]
fn check_accepts_nullable_alias_weak_field_type() {
    let (_source, _graph, result) = check_source(
        r#"
type MaybeNode = Node | null

struct Node {
  weak parent: MaybeNode,
}
"#,
    );

    assert_eq!(result.ownership_metadata().weak_fields().len(), 1);
}

#[test]
fn check_accepts_inferred_struct_literal_with_expected_type() {
    let (_source, _graph, result) = check_source(
        r#"
        struct User {
          id: int32,
          name: [uint8],
        }

        var user: User = struct {
          id: 1,
          name: "Ana",
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_inferred_struct_literal_with_default_field() {
    let (_source, _graph, result) = check_source(
        r#"
        struct User {
          name: [uint8],
          age: int32 = 0,
        }

        var user: User = struct {
          name: "Ana",
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_inferred_struct_literal_without_expected_type() {
    let source = source(
        r#"
        var user = struct {
          id: 1,
        }
        "#,
    );

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

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::CannotInferType.as_code()
            && diagnostic
                .message()
                .contains("inferred struct literal requires an expected struct type")
    }));
}

#[test]
fn check_reports_inferred_struct_literal_with_non_struct_expected_type() {
    let source = source(
        r#"
        var value: int32 = struct {
          id: 1,
        }
        "#,
    );

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

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::CannotInferType.as_code()
            && diagnostic
                .message()
                .contains("inferred struct literal requires an expected struct type")
    }));
}

#[test]
fn check_reports_inferred_struct_literal_unknown_field() {
    let source = source(
        r#"
        struct User {
          id: int32,
        }

        var user: User = struct {
          id: 1,
          name: "Ana",
        }
        "#,
    );

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

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::UnknownStructField.as_code()
            && diagnostic.message().contains("has no field `name`")
    }));
}

#[test]
fn check_reports_inferred_struct_literal_field_type_mismatch() {
    let source = source(
        r#"
        struct User {
          id: int32,
        }

        var user: User = struct {
          id: true,
        }
        "#,
    );

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

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::TypeMismatch.as_code()
            && diagnostic
                .message()
                .contains("expected `int32`, got `bool`")
    }));
}
