use super::*;

#[test]
fn check_accepts_primitive_type_alias_assignment() {
    let (_source, _graph, result) = check_source(
        r#"
type UserId = int32

var id: UserId = 1
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_primitive_type_alias_assignment_mismatch() {
    let source = source(
        r#"
type UserId = int32

var id: UserId = true
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
fn check_accepts_union_type_alias_assignment() {
    let (_source, _graph, result) = check_source(
        r#"
type MaybeInt = int32 | null

var value: MaybeInt = null
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_dynamic_array_type_alias_assignment() {
    let (_source, _graph, result) = check_source(
        r#"
type Bytes = [uint8]

var name: Bytes = ""
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_fixed_array_type_alias_assignment() {
    let (_source, _graph, result) = check_source(
        r#"
type Triple = [int32; 3]

var values: Triple = [1, 2, 3]
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_fixed_array_type_alias_size_mismatch() {
    let source = source(
        r#"
type Pair = [int32; 2]

var values: Pair = [1, 2, 3]
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
                .contains("expected `[int32; 2]`, got `[int32; 3]`")
    }));
}

#[test]
fn check_accepts_for_over_dynamic_array_type_alias() {
    let (_source, _graph, result) = check_source(
        r#"
type Ints = [int32]

fn main(values: Ints): null {
  for value in values {
    var copied: int32 = value
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_struct_field_type_alias() {
    let (_source, _graph, result) = check_source(
        r#"
type Bytes = [uint8]

struct User {
  name: Bytes,
}

var user: User = User {
  name: "Ana",
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_instanceof_with_union_type_alias() {
    let (_source, _graph, result) = check_source(
        r#"
type MaybeInt = int32 | null

fn normalize(value: MaybeInt): int32 {
  return instanceof value {
    int32 number => number,
    _ => 0,
  }
}
"#,
    );

    assert!(!result.has_errors());
}
