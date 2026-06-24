use super::*;

#[test]
fn check_accepts_function_decorator_without_resolving_target() {
    let (_source, _graph, result) = check_source(
        r#"
        @log
        fn save(): null {
          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_decorator_call_without_resolving_target() {
    let (_source, _graph, result) = check_source(
        r#"
        @tag("stable", 1, true)
        fn save(): null {
          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_struct_and_field_decorators() {
    let (_source, _graph, result) = check_source(
        r#"
        @frozen
        struct User {
          @trim
          name: [uint8],
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_parameter_and_rest_parameter_decorators() {
    let (_source, _graph, result) = check_source(
        r#"
        fn save(@trim name: [uint8], @nonempty ...values: [int32]): null {
          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_choice_payload_item_decorator() {
    let (_source, _graph, result) = check_source(
        r#"
        choice Asset {
          Texture(@path [uint8]),
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_weak_struct_field_decorator() {
    let (_source, _graph, result) = check_source(
        r#"
        struct Node {
          @nullable
          weak parent: Node | null,
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_omitted_decorator_argument() {
    let source = source(
        r#"
        @tag(,1)
        fn save(): null {
          return
        }
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidDecoratorUsage.as_code()
            && diagnostic
                .message()
                .contains("decorator arguments cannot be omitted")
    }));
}

#[test]
fn parse_accepts_decorated_weak_struct_field() {
    let source = source(
        r#"
        struct Node {
          @nullable
          weak parent: Node | null,
        }
        "#,
    );

    let result = parse(&source);

    assert!(!result.has_errors(), "{:?}", result.diagnostics());

    let graph = result.into_graph();
    let weak_field = find_node_by_kind(&graph, SyntaxNodeKind::WeakStructField).unwrap();

    assert!(
        graph
            .syntax()
            .first_child_of_kind(weak_field, SyntaxNodeKind::DecoratorList)
            .is_some()
    );
}
