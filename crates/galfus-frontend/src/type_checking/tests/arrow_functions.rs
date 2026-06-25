use super::*;

#[test]
fn check_infers_arrow_function_expression_body_type() {
    let (source, graph, result) = check_source(
        r#"
        var double = (value: int32): int32 => value * 2
        "#,
    );

    let arrow = find_node_by_kind(&graph, SyntaxNodeKind::ArrowFunctionExpression).unwrap();

    let ty = result.layer().node_type(arrow).unwrap();

    let TypeKind::Function(function) = result.layer().table().kind(ty).unwrap() else {
        panic!("expected function type");
    };

    assert_eq!(function.parameters().len(), 1);

    assert_eq!(
        result.layer().table().kind(function.parameters()[0].ty()),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );

    assert_eq!(
        result.layer().table().kind(function.return_type()),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );

    assert_eq!(
        source.slice(graph.syntax().node(arrow).unwrap().span()),
        Some("(value: int32): int32 => value * 2")
    );
}

#[test]
fn check_infers_arrow_function_return_type_without_annotation() {
    let (_source, graph, result) = check_source(
        r#"
        var double = (value: int32) => value * 2
        "#,
    );

    let arrow = find_node_by_kind(&graph, SyntaxNodeKind::ArrowFunctionExpression).unwrap();

    let ty = result.layer().node_type(arrow).unwrap();

    let TypeKind::Function(function) = result.layer().table().kind(ty).unwrap() else {
        panic!("expected function type");
    };

    assert_eq!(
        result.layer().table().kind(function.return_type()),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_accepts_arrow_function_block_body() {
    let (_source, _graph, result) = check_source(
        r#"
        var printer = (value: int32): null => {
          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_arrow_function_as_call_argument() {
    let (_source, _graph, result) = check_source(
        r#"
        fn apply(callback: fn(int32): int32): int32 {
          return callback(1)
        }

        var result = apply((value: int32): int32 => value * 2)
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_collects_closure_capture_ownership_metadata() {
    let (_source, graph, result) = check_source(
        r#"
        struct Box {
          value: int32,
        }

        var captured: Box = Box { value: 2 }
        var make = (): Box => captured
        "#,
    );

    let arrow = find_node_by_kind(&graph, SyntaxNodeKind::ArrowFunctionExpression).unwrap();
    let captured = symbol_by_name_and_kind(&graph, "captured", SymbolKind::Var);

    let captures = result.ownership_metadata().captures();

    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].closure(), arrow);
    assert_eq!(captures[0].symbol(), captured);

    assert!(
        result
            .ownership_metadata()
            .release_eligibilities()
            .iter()
            .any(|eligibility| {
                eligibility.kind() == ReleaseEligibilityKind::Capture
                    && eligibility.symbol() == Some(captured)
            })
    );
}

#[test]
fn check_does_not_capture_arrow_local_parameter() {
    let (_source, _graph, result) = check_source(
        r#"
        var double = (value: int32): int32 => value * 2
        "#,
    );

    assert!(result.ownership_metadata().captures().is_empty());
}

#[test]
fn check_does_not_leak_arrow_block_return_to_outer_function() {
    let (_source, _graph, result) = check_source(
        r#"
        fn main(): null {
          var callback = (value: int32): int32 => {
            return value
          }

          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_arrow_function_expression_body_return_mismatch() {
    let source = source(
        r#"
        var bad = (value: int32): bool => value * 2
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
                .contains("expected `bool`, got `int32`")
    }));
}

#[test]
fn check_reports_arrow_function_assignment_mismatch() {
    let source = source(
        r#"
        var callback: fn(int32): bool = (value: int32): int32 => value * 2
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
    }));
}
