use super::*;

#[test]
fn check_accepts_matching_var_initializer_type() {
    let (_source, _graph, result) = check_source(
        r#"
var age: int32 = 10
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_var_initializer_type_mismatch() {
    let source = source(
        r#"
var age: int32 = true
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
fn check_accepts_matching_const_initializer_type() {
    let (_source, _graph, result) = check_source(
        r#"
const enabled: bool = true
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_null_initializer_for_nullable_union() {
    let (_source, _graph, result) = check_source(
        r#"
var maybe: int32 | null = null
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_null_initializer_for_non_nullable_type() {
    let source = source(
        r#"
var age: int32 = null
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
                .contains("expected `int32`, got `null`")
    }));
}

#[test]
fn check_reports_top_level_initialization_cycle() {
    let source = source(
        r#"
var a = b
var b = a
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InitializationCycle.as_code()
            && diagnostic.message().contains("initialization cycle")
    }));
}

#[test]
fn check_reports_local_initialization_cycle() {
    let source = source(
        r#"
fn main(): null {
  var a = b
  var b = a
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InitializationCycle.as_code()
    }));
}

#[test]
fn check_accepts_name_initializer_with_matching_symbol_type() {
    let (_source, _graph, result) = check_source(
        r#"
var first: int32 = 10
var second: int32 = first
"#,
    );

    assert!(!result.has_errors());
}
