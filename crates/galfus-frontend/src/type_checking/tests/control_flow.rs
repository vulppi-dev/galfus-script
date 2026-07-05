use super::*;

#[test]
fn check_accepts_if_bool_condition() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  if true {
    return
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_if_non_bool_condition() {
    let source = source(
        r#"
fn main(): null {
  if 1 {
    return
  }

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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidConditionType.as_code()
            && diagnostic
                .message()
                .contains("condition must be `bool`, got `int32`")
    }));
}

#[test]
fn check_accepts_break_inside_loop() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  loop {
    break
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_continue_inside_loop() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  loop {
    continue
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_break_outside_loop() {
    let source = source(
        r#"
fn main(): null {
  break
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
        diagnostic.code().as_str() == TypeDiagnosticCode::BreakOutsideLoop.as_code()
            && diagnostic
                .message()
                .contains("`break` can only be used inside a loop")
    }));
}

#[test]
fn check_reports_continue_outside_loop() {
    let source = source(
        r#"
fn main(): null {
  continue
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
        diagnostic.code().as_str() == TypeDiagnosticCode::ContinueOutsideLoop.as_code()
            && diagnostic
                .message()
                .contains("`continue` can only be used inside a loop")
    }));
}
