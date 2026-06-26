use super::*;

#[test]
fn check_accepts_struct_literal() {
    let (_source, _graph, result) = check_source(
        r#"
struct User {
  id: int32,
  name: [uint8],
}

var user: User = new(User) {
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

var user: User = new(User) {
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

var user: User = new(User) {
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

var user: User = new(User) {
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

var user: User = new(User) {
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

var user: User = new(User) {
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

var user: User = new(User) { id: 1 }

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

var user: User = new(User) { id: 1 }

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

var user: User = new(User) {
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

var user: User = new(User) {
  id,
  name,
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_struct_literal_spread_from_same_struct() {
    let (_source, _graph, result) = check_source(
        r#"
struct User {
  id: int32,
  name: [uint8],
}

var base: User = new(User) {
  id: 1,
  name: "Ana",
}

var renamed: User = new(User) {
  ...base,
  name: "Bia",
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_invalid_struct_literal_spread_target() {
    let source = source(
        r#"
struct User {
  id: int32,
}

var base: int32 = 1
var user: User = new(User) {
  ...base,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidStructSpreadTarget.as_code()
            && diagnostic
                .message()
                .contains("struct literal spread target must be a struct")
    }));
}

#[test]
fn check_reports_invalid_struct_expansion_target() {
    let source = source(
        r#"
struct User {
  ...int32,
  id: int32,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidStructExpansionTarget.as_code()
            && diagnostic
                .message()
                .contains("struct expansion target must be a struct")
    }));
}

#[test]
fn check_accepts_struct_expansion_fields_in_literal() {
    let (_source, _graph, result) = check_source(
        r#"
struct User {
  id: int32,
  name: [uint8],
}

struct Admin {
  ...User,
  role: [uint8],
}

var admin: Admin = new(Admin) {
  id: 1,
  name: "Ana",
  role: "root",
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_member_access_to_struct_expansion_field() {
    let (_source, _graph, result) = check_source(
        r#"
struct User {
  id: int32,
}

struct Admin {
  ...User,
  role: [uint8],
}

var admin: Admin = new(Admin) {
  id: 1,
  role: "root",
}

var id: int32 = admin.id
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_binds_copy_expression_to_original_value_type() {
    let (source, graph, result) = check_source(
        r#"
struct User {
  id: int32,
}

var user: User = new(User) { id: 1 }
var cloned: User = copy user
"#,
    );

    let copy =
        find_node_by_kind_and_text(&source, &graph, SyntaxNodeKind::CopyExpression, "copy user")
            .unwrap();

    let user_symbol = symbol_by_name_and_kind(&graph, "User", SymbolKind::Struct);
    let ty = result.layer().node_type(copy).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Named {
            symbol: user_symbol
        })
    );
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
fn check_collects_edge_and_weak_observer_ownership_metadata() {
    let (_source, graph, result) = check_source(
        r#"
struct Node {
  child: Node | null,
  weak parent: Node | null,
}
"#,
    );

    let node_symbol = symbol_by_name_and_kind(&graph, "Node", SymbolKind::Struct);
    let child_symbol = symbol_by_name_and_kind(&graph, "child", SymbolKind::StructField);
    let parent_symbol = symbol_by_name_and_kind(&graph, "parent", SymbolKind::StructField);

    assert!(
        result.ownership_metadata().edges().iter().any(|edge| {
            edge.owner_symbol() == node_symbol && edge.field_symbol() == child_symbol
        })
    );

    assert!(
        result
            .ownership_metadata()
            .weak_observers()
            .iter()
            .any(|observer| {
                observer.owner_symbol() == node_symbol && observer.field_symbol() == parent_symbol
            })
    );
}

#[test]
fn check_collects_strong_ownership_cycle_metadata() {
    let (_source, graph, result) = check_source(
        r#"
struct Parent {
  child: Child | null,
}

struct Child {
  parent: Parent | null,
}
"#,
    );

    let parent_symbol = symbol_by_name_and_kind(&graph, "Parent", SymbolKind::Struct);
    let child_symbol = symbol_by_name_and_kind(&graph, "Child", SymbolKind::Struct);

    assert!(result.ownership_metadata().cycles().iter().any(|cycle| {
        cycle.structs().contains(&parent_symbol) && cycle.structs().contains(&child_symbol)
    }));
}

#[test]
fn check_ignores_weak_fields_for_ownership_cycle_metadata() {
    let (_source, _graph, result) = check_source(
        r#"
struct Parent {
  child: Child | null,
}

struct Child {
  weak parent: Parent | null,
}
"#,
    );

    assert!(result.ownership_metadata().cycles().is_empty());
}

#[test]
fn check_collects_anchor_and_temporary_ownership_metadata() {
    let (_source, graph, result) = check_source(
        r#"
struct User {
  name: [uint8],
}

var global_user: User = new(User) { name: "Ana" }

fn User::rename(user: User, name: [uint8]): User {
  var local_user: User = new(User) { name }
  return local_user
}
"#,
    );

    let global_user = symbol_by_name_and_kind(&graph, "global_user", SymbolKind::Var);
    let local_user = symbol_by_name_and_kind(&graph, "local_user", SymbolKind::Var);
    let user_parameter = symbol_by_name_and_kind(&graph, "user", SymbolKind::Parameter);

    let anchors = result.ownership_metadata().anchors();

    assert!(anchors.iter().any(|anchor| {
        anchor.kind() == AnchorKind::ModuleState && anchor.symbol() == Some(global_user)
    }));

    assert!(anchors.iter().any(|anchor| {
        anchor.kind() == AnchorKind::BlockLocal && anchor.symbol() == Some(local_user)
    }));

    assert!(anchors.iter().any(|anchor| {
        anchor.kind() == AnchorKind::FunctionParameter && anchor.symbol() == Some(user_parameter)
    }));

    assert!(
        anchors
            .iter()
            .any(|anchor| anchor.kind() == AnchorKind::FunctionAnchor)
    );

    assert!(!result.ownership_metadata().temporaries().is_empty());
    assert!(
        anchors
            .iter()
            .any(|anchor| anchor.kind() == AnchorKind::Temporary)
    );

    assert!(
        result
            .ownership_metadata()
            .release_eligibilities()
            .iter()
            .any(|eligibility| {
                eligibility.kind() == ReleaseEligibilityKind::Anchor
                    && eligibility.symbol() == Some(global_user)
            })
    );

    assert!(
        result
            .ownership_metadata()
            .release_eligibilities()
            .iter()
            .any(|eligibility| eligibility.kind() == ReleaseEligibilityKind::Temporary)
    );
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

        var user: User = new {
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

        var user: User = new {
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
        var user = new {
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
        var value: int32 = new {
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

        var user: User = new {
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

        var user: User = new {
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
