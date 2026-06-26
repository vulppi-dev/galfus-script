use super::*;

#[test]
fn check_accepts_explicit_generic_function_call() {
    let (_source, _graph, result) = check_source(
        r#"
        fn identity<T>(value: T): T {
          return value
        }

        var value: int32 = identity<int32>(1)
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_binds_explicit_generic_call_return_type() {
    let (source, graph, result) = check_source(
        r#"
        fn identity<T>(value: T): T {
          return value
        }

        var value = identity<int32>(1)
        "#,
    );

    let call = find_node_by_kind_and_text(
        &source,
        &graph,
        SyntaxNodeKind::CallExpression,
        "identity<int32>(1)",
    )
    .unwrap();

    let ty = result.layer().node_type(call).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_accepts_explicit_generic_function_call_with_array_type_argument() {
    let (_source, _graph, result) = check_source(
        r#"
        fn identity<T>(value: T): T {
          return value
        }

        var bytes: [uint8] = "Ana"
        var value: [uint8] = identity<[uint8]>(bytes)
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_explicit_generic_function_call_with_multiple_arguments() {
    let (_source, _graph, result) = check_source(
        r#"
        fn first<A, B>(left: A, right: B): A {
          return left
        }

        var value: int32 = first<int32, bool>(1, true)
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_explicit_generic_argument_count_mismatch_for_missing_argument() {
    let source = source(
        r#"
        fn pair<A, B>(left: A, right: B): A {
          return left
        }

        var value = pair<int32>(1, true)
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
        diagnostic.code().as_str() == TypeDiagnosticCode::GenericArgumentCountMismatch.as_code()
            && diagnostic
                .message()
                .contains("expected 2 generic argument(s), got 1")
    }));
}

#[test]
fn check_reports_explicit_generic_argument_count_mismatch_for_extra_argument() {
    let source = source(
        r#"
        fn identity<T>(value: T): T {
          return value
        }

        var value = identity<int32, bool>(1)
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
        diagnostic.code().as_str() == TypeDiagnosticCode::GenericArgumentCountMismatch.as_code()
            && diagnostic
                .message()
                .contains("expected 1 generic argument(s), got 2")
    }));
}

#[test]
fn check_reports_explicit_generic_call_argument_type_mismatch_after_substitution() {
    let source = source(
        r#"
        fn identity<T>(value: T): T {
          return value
        }

        var value = identity<int32>(true)
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

#[test]
fn check_reports_generic_arguments_on_non_generic_function() {
    let source = source(
        r#"
        fn one(): int32 {
          return 1
        }

        var value = one<int32>()
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
        diagnostic.code().as_str() == TypeDiagnosticCode::GenericArgumentCountMismatch.as_code()
            && diagnostic
                .message()
                .contains("expected 0 generic argument(s), got 1")
    }));
}
