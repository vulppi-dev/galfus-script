use super::*;

#[test]
fn check_accepts_instanceof_type_pattern_binding() {
    let (_source, _graph, result) = check_source(
        r#"
fn normalize(value: int32 | null): int32 {
  return instanceof value {
    int32 number => number,
    _ => 0,
  }
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_binds_instanceof_type_pattern_binding_type() {
    let (source, graph, result) = check_source(
        r#"
fn normalize(value: int32 | null): int32 {
  return instanceof value {
    int32 number => number,
    _ => 0,
  }
}
"#,
    );

    let binding = find_node_by_kind_and_text(
        &source,
        &graph,
        SyntaxNodeKind::TypePatternBinding,
        "number",
    )
    .unwrap();

    let ty = result.layer().node_type(binding).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_accepts_instanceof_parenthesized_type_pattern_binding() {
    let (_source, _graph, result) = check_source(
        r#"
fn normalize(value: int32 | null): int32 {
  return instanceof value {
    int32(number) => number,
    _ => 0,
  }
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_instanceof_wildcard_pattern() {
    let (_source, _graph, result) = check_source(
        r#"
fn normalize(value: int32): int32 {
  return instanceof value {
    _ => 0,
  }
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_instanceof_pattern_type_mismatch() {
    let source = source(
        r#"
fn normalize(value: int32): int32 {
  return instanceof value {
    bool flag => 1,
    _ => 0,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidInstanceofPatternType.as_code()
            && diagnostic.message().contains("got `bool`")
    }));
}

#[test]
fn check_reports_incompatible_instanceof_arm_type() {
    let source = source(
        r#"
fn normalize(value: int32 | null): int32 {
  return instanceof value {
    int32 number => true,
    _ => 0,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::IncompatibleInstanceofArmType.as_code()
            && diagnostic
                .message()
                .contains("instanceof arm body must be compatible with `bool`, got `int32`")
    }));
}

#[test]
fn check_accepts_instanceof_binding_pattern() {
    let (_source, _graph, result) = check_source(
        r#"
fn normalize(value: int32): int32 {
  return instanceof value {
    other => other,
  }
}
"#,
    );

    assert!(!result.has_errors());
}
