use super::*;

#[test]
fn check_accepts_typeof_over_generic_bound() {
    let (_source, _graph, result) = check_source(
        r#"
fn stringify<T: int | uint | bool | null | [uint8]>(value: T): [uint8] {
  return typeof T {
    int => "int",
    uint => "uint",
    bool => "bool",
    null => "null",
    [uint8] => value,
  }
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_typeof_narrows_generic_value_in_arm() {
    let (_source, _graph, result) = check_source(
        r#"
fn identity_text<T: int | [uint8]>(value: T): [uint8] {
  return typeof T {
    int => "number",
    [uint8] => value,
  }
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_typeof_struct_choice_and_enum_patterns() {
    let (_source, _graph, result) = check_source(
        r#"
struct User {
  name: [uint8],
}

choice Result {
  Ok(User),
  Err([uint8]),
}

enum State {
  Ready,
}

fn label_user(): [uint8] {
  return typeof User {
    User => "user",
  }
}

fn label_result(): [uint8] {
  return typeof Result {
    Result => "result",
  }
}

fn label_state(): [uint8] {
  return typeof State {
    State => "state",
  }
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_typeof_pattern_outside_bound() {
    let source = source(
        r#"
fn stringify<T: int | bool>(value: T): [uint8] {
  return typeof T {
    float => "float",
    _ => "other",
  }
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidTypeofPatternType.as_code()
            && diagnostic.message().contains("got `float")
    }));
}
