use super::*;

#[test]
fn check_accepts_numeric_binary_expression() {
    let (_source, _graph, result) = check_source(
        r#"
var value: int32 = 1 + 2
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_numeric_binary_expression_type_error() {
    let source = source(
        r#"
var value: int32 = 1 + true
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
        diagnostic.code().as_str() == TypeDiagnosticCode::UnsupportedOperator.as_code()
            && diagnostic.message().contains("numeric operands")
    }));
}

#[test]
fn check_accepts_numeric_comparison_expression() {
    let (_source, _graph, result) = check_source(
        r#"
var value: bool = 1 < 2
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_equality_expression() {
    let (_source, _graph, result) = check_source(
        r#"
var value: bool = 1 == 2
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_equality_type_error() {
    let source = source(
        r#"
var value: bool = 1 == true
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
        diagnostic.code().as_str() == TypeDiagnosticCode::UnsupportedOperator.as_code()
            && diagnostic.message().contains("comparable operands")
    }));
}

#[test]
fn check_accepts_bool_binary_expression() {
    let (_source, _graph, result) = check_source(
        r#"
var value: bool = true && false
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_bool_binary_expression_type_error() {
    let source = source(
        r#"
var value: bool = true && 1
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
        diagnostic.code().as_str() == TypeDiagnosticCode::UnsupportedOperator.as_code()
            && diagnostic.message().contains("bool operands")
    }));
}

#[test]
fn check_accepts_numeric_unary_expression() {
    let (_source, _graph, result) = check_source(
        r#"
var value: int32 = -1
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_bool_unary_expression() {
    let (_source, _graph, result) = check_source(
        r#"
var value: bool = !true
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_bool_unary_expression_type_error() {
    let source = source(
        r#"
var value: bool = !1
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
        diagnostic.code().as_str() == TypeDiagnosticCode::UnsupportedOperator.as_code()
            && diagnostic.message().contains("bool operand")
    }));
}

#[test]
fn check_accepts_integer_bitwise_expression() {
    let (_source, _graph, result) = check_source(
        r#"
var value: int32 = 1 & 2
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_integer_shift_expression() {
    let (_source, _graph, result) = check_source(
        r#"
var value: int32 = 1 << 2
"#,
    );

    assert!(!result.has_errors());
}
