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

#[test]
fn check_accepts_struct_satisfies_constraint_function() {
    let (_source, _graph, result) = check_source(
        r#"
constraint Named {
  fn name(): [uint8],
}

struct User satisfies Named {
  name: [uint8],
}

fn User::name(): [uint8] {
  return "Ana"
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_missing_constraint_function() {
    let source = source(
        r#"
constraint Named {
  fn name(): [uint8],
}

struct User satisfies Named {
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
        diagnostic.code().as_str() == TypeDiagnosticCode::MissingConstraintFunction.as_code()
            && diagnostic.message().contains("missing function `name`")
    }));
}

#[test]
fn check_reports_constraint_function_return_type_mismatch() {
    let source = source(
        r#"
constraint Named {
  fn name(): [uint8],
}

struct User satisfies Named {
  name: [uint8],
}

fn User::name(): int32 {
  return 1
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
        diagnostic.code().as_str() == TypeDiagnosticCode::ConstraintFunctionTypeMismatch.as_code()
            && diagnostic.message().contains("function `name`")
    }));
}

#[test]
fn check_reports_constraint_function_parameter_type_mismatch() {
    let source = source(
        r#"
constraint Setter {
  fn set(value: [uint8]): null,
}

struct User satisfies Setter {
  name: [uint8],
}

fn User::set(value: int32): null {
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
        diagnostic.code().as_str() == TypeDiagnosticCode::ConstraintFunctionTypeMismatch.as_code()
            && diagnostic.message().contains("function `set`")
    }));
}

#[test]
fn check_accepts_generic_constraint_function_explicit_argument() {
    let (_source, _graph, result) = check_source(
        r#"
constraint Stringable<T> {
  fn toString(self: T): [uint8],
}

struct User satisfies Stringable<User> {
  name: [uint8],
}

fn User::toString(self: User): [uint8] {
  return self.name
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_generic_constraint_function_self_type_mismatch() {
    let source = source(
        r#"
constraint Stringable<T> {
  fn toString(self: T): [uint8],
}

struct User satisfies Stringable<User> {
  name: [uint8],
}

fn User::toString(self: int32): [uint8] {
  return "invalid"
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
        diagnostic.code().as_str() == TypeDiagnosticCode::ConstraintFunctionTypeMismatch.as_code()
            && diagnostic.message().contains("function `toString`")
    }));
}

#[test]
fn check_accepts_generic_constraint_field() {
    let (_source, _graph, result) = check_source(
        r#"
constraint HasValue<T> {
  value: T,
}

struct IntBox satisfies HasValue<int32> {
  value: int32,
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_generic_constraint_field_type_mismatch() {
    let source = source(
        r#"
constraint HasValue<T> {
  value: T,
}

struct IntBox satisfies HasValue<int32> {
  value: bool,
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
            && diagnostic.message().contains("field `value`")
    }));
}

#[test]
fn check_reports_generic_constraint_missing_argument() {
    let source = source(
        r#"
constraint Stringable<T> {
  fn toString(self: T): [uint8],
}

struct User satisfies Stringable {
  name: [uint8],
}

fn User::toString(self: User): [uint8] {
  return self.name
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
        diagnostic.code().as_str()
            == TypeDiagnosticCode::ConstraintGenericArgumentCountMismatch.as_code()
            && diagnostic
                .message()
                .contains("constraint `Stringable` expects 1 generic argument")
    }));
}

#[test]
fn check_accepts_generic_parameter_constraint_bound() {
    let (_source, _graph, result) = check_source(
        r#"
        constraint HasValue<T> {
          value: T,
        }

        fn read<T: HasValue<int32>>(value: T): null {
          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_generic_parameter_constraint_non_constraint_target() {
    let source = source(
        r#"
        struct Other {
          value: int32,
        }

        fn read<T: Other>(value: T): null {
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidSatisfiesTarget.as_code()
            && diagnostic
                .message()
                .contains("satisfies target `Other` is not a constraint")
    }));
}

#[test]
fn check_reports_generic_parameter_constraint_missing_argument() {
    let source = source(
        r#"
        constraint HasValue<T> {
          value: T,
        }

        fn read<T: HasValue>(value: T): null {
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
        diagnostic.code().as_str()
            == TypeDiagnosticCode::ConstraintGenericArgumentCountMismatch.as_code()
            && diagnostic
                .message()
                .contains("constraint `HasValue` expects 1 generic argument")
    }));
}

#[test]
fn check_reports_generic_parameter_constraint_extra_argument() {
    let source = source(
        r#"
        constraint HasValue<T> {
          value: T,
        }

        fn read<T: HasValue<int32, bool>>(value: T): null {
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
        diagnostic.code().as_str()
            == TypeDiagnosticCode::ConstraintGenericArgumentCountMismatch.as_code()
            && diagnostic
                .message()
                .contains("constraint `HasValue` expects 1 generic argument")
    }));
}
