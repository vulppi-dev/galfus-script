use super::*;

#[test]
fn check_accepts_match_literal_patterns() {
    let (_source, _graph, result) = check_source(
        r#"
fn code(value: int32): int32 {
  return match value {
    1 => 10,
    2 => 20,
    fallback => fallback,
  }
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_match_literal_pattern_type_mismatch() {
    let source = source(
        r#"
fn code(value: int32): int32 {
  return match value {
    true => 1,
    fallback => fallback,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidMatchPatternType.as_code()
            && diagnostic.message().contains("got `bool`")
    }));
}

#[test]
fn check_reports_incompatible_match_arm_type() {
    let source = source(
        r#"
fn code(value: int32): int32 {
  return match value {
    1 => 10,
    fallback => true,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::IncompatibleMatchArmType.as_code()
            && diagnostic
                .message()
                .contains("match arm body must be compatible with `int32`, got `bool`")
    }));
}

#[test]
fn check_accepts_enum_variant_patterns() {
    let (_source, _graph, result) = check_source(
        r#"
enum Direction {
  North,
  South,
}

fn code(direction: Direction): int32 {
  return match direction {
    Direction::North => 1,
    Direction::South => 2,
  }
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_choice_payload_pattern() {
    let (_source, _graph, result) = check_source(
        r#"
choice Result {
  Ok(int32),
  Err([uint8]),
}

fn unwrap(result: Result): int32 {
  return match result {
    Result::Ok(value) => value,
    Result::Err(message) => 0,
  }
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_choice_payload_pattern_count_mismatch() {
    let source = source(
        r#"
choice Result {
  Ok(int32),
}

fn unwrap(result: Result): int32 {
  return match result {
    Result::Ok() => 0,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::ArgumentCountMismatch.as_code()
            && diagnostic.message().contains("expected 1 arguments, got 0")
    }));
}

#[test]
fn check_reports_choice_payload_pattern_type_mismatch() {
    let source = source(
        r#"
choice Result {
  Ok(int32),
}

fn unwrap(result: Result): int32 {
  return match result {
    Result::Ok(true) => 0,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidMatchPatternType.as_code()
            && diagnostic.message().contains("got `bool`")
    }));
}
