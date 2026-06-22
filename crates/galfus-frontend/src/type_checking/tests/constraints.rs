use super::*;

#[test]
fn check_accepts_struct_satisfies_constraint_field() {
    let (_source, _graph, result) = check_source(
        r#"
constraint Named {
  name: [uint8],
}

struct User satisfies Named {
  name: [uint8],
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_satisfies_non_constraint_target() {
    let source = source(
        r#"
struct Other {
  name: [uint8],
}

struct User satisfies Other {
  name: [uint8],
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidSatisfiesTarget.as_code()
            && diagnostic
                .message()
                .contains("satisfies target `Other` is not a constraint")
    }));
}

#[test]
fn check_reports_missing_constraint_field() {
    let source = source(
        r#"
constraint Named {
  name: [uint8],
}

struct User satisfies Named {
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
        diagnostic.code().as_str() == TypeDiagnosticCode::MissingConstraintField.as_code()
            && diagnostic.message().contains("missing field `name`")
    }));
}

#[test]
fn check_reports_constraint_field_type_mismatch() {
    let source = source(
        r#"
constraint Named {
  name: [uint8],
}

struct User satisfies Named {
  name: int32,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::ConstraintFieldTypeMismatch.as_code()
            && diagnostic
                .message()
                .contains("field `name` expected `[uint8]`, got `int32`")
    }));
}

#[test]
fn check_accepts_struct_satisfies_multiple_constraints() {
    let (_source, _graph, result) = check_source(
        r#"
constraint Named {
  name: [uint8],
}

constraint Identified {
  id: int32,
}

struct User satisfies Named, Identified {
  id: int32,
  name: [uint8],
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_constraint_field_type_alias() {
    let (_source, _graph, result) = check_source(
        r#"
type Bytes = [uint8]

constraint Named {
  name: Bytes,
}

struct User satisfies Named {
  name: [uint8],
}
"#,
    );

    assert!(!result.has_errors());
}
