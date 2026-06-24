use super::*;

#[test]
fn check_accepts_call_with_matching_arguments() {
    let (_source, _graph, result) = check_source(
        r#"
fn add(a: int32, b: int32): int32 {
  return a
}

var value: int32 = add(1, 2)
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_binds_call_expression_return_type() {
    let (_source, graph, result) = check_source(
        r#"
fn one(): int32 {
  return 1
}

var value: int32 = one()
"#,
    );

    let source = SourceFile::new(
        galfus_core::SourceId::new(0),
        "test.gfs".to_string(),
        r#"
fn one(): int32 {
  return 1
}

var value: int32 = one()
"#
        .to_string(),
    );

    let call = find_node_by_kind_and_text(&source, &graph, SyntaxNodeKind::CallExpression, "one()")
        .unwrap();

    let ty = result.layer().node_type(call).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_reports_call_argument_type_mismatch() {
    let source = source(
        r#"
fn add(a: int32, b: int32): int32 {
  return a
}

var value: int32 = add(true, 2)
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
fn check_reports_too_few_call_arguments() {
    let source = source(
        r#"
fn add(a: int32, b: int32): int32 {
  return a
}

var value: int32 = add(1)
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
            && diagnostic.message().contains("expected 2 arguments, got 1")
    }));
}

#[test]
fn check_reports_too_many_call_arguments() {
    let source = source(
        r#"
fn add(a: int32, b: int32): int32 {
  return a
}

var value: int32 = add(1, 2, 3)
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
            && diagnostic.message().contains("expected 2 arguments, got 3")
    }));
}

#[test]
fn check_reports_calling_non_function() {
    let source = source(
        r#"
var age: int32 = 10
var result: int32 = age()
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
        diagnostic.code().as_str() == TypeDiagnosticCode::NotCallable.as_code()
            && diagnostic.message().contains("is not callable")
    }));
}

#[test]
fn check_accepts_default_parameter_argument_count() {
    let (_source, _graph, result) = check_source(
        r#"
fn add(a: int32, b: int32 = 1): int32 {
  return a
}

var value: int32 = add(1)
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_rest_parameter_argument_count() {
    let (_source, _graph, result) = check_source(
        r#"
fn sum(...values: [int32]): int32 {
  return 1
}

var value: int32 = sum(1, 2, 3)
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_omitted_default_argument() {
    let (_source, _graph, result) = check_source(
        r#"
        fn call(a: int32, b: int32 = 2, c: int32 = 3): int32 {
          return a + b + c
        }

        var value: int32 = call(1,,3)
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_omitted_required_argument() {
    let source = source(
        r#"
        fn call(a: int32, b: int32 = 2): int32 {
          return a + b
        }

        var value: int32 = call(,2)
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
        diagnostic.code().as_str() == TypeDiagnosticCode::ArgumentCountMismatch.as_code()
            && diagnostic.message().contains("argument cannot be omitted")
    }));
}

#[test]
fn check_accepts_individual_arguments_for_rest_parameter() {
    let (_source, _graph, result) = check_source(
        r#"
        fn sum(...values: [int32]): int32 {
          return 0
        }

        fn call(...values: [int32]): null {
          return
        }

        var value: int32 = sum(1, 2, 3)

        fn main(): null {
          call()
          call(1)
          call(1, 2)
          call(1, 2, 3)
          return
        }
        "#,
    );

    assert!(!result.has_errors());
}
