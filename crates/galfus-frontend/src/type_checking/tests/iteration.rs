use super::*;

#[test]
fn check_accepts_for_over_dynamic_array() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(values: [int32]): null {
  for value in values {
    var copied: int32 = value
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_for_over_fixed_array_literal() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  for value in [1, 2, 3] {
    var copied: int32 = value
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_for_over_string_literal_as_uint8_array() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  for byte in "Ana" {
    var copied: uint8 = byte
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_for_over_empty_string_literal() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  for byte in "" {
    var copied: uint8 = byte
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_for_over_non_array() {
    let source = source(
        r#"
fn main(): null {
  for value in 10 {
    var copied: int32 = value
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidIterableType.as_code()
            && diagnostic
                .message()
                .contains("for iterable must be an array, got `int32`")
    }));
}

#[test]
fn check_binds_for_binding_type_from_dynamic_array() {
    let (source, graph, result) = check_source(
        r#"
fn main(values: [int32]): null {
  for value in values {
    var copied: int32 = value
  }

  return
}
"#,
    );

    let binding =
        find_node_by_kind_and_text(&source, &graph, SyntaxNodeKind::ForBinding, "value").unwrap();

    let ty = result.layer().node_type(binding).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_reports_for_binding_type_mismatch_in_body() {
    let source = source(
        r#"
fn main(values: [int32]): null {
  for value in values {
    var copied: bool = value
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
        diagnostic.code().as_str() == TypeDiagnosticCode::TypeMismatch.as_code()
            && diagnostic
                .message()
                .contains("expected `bool`, got `int32`")
    }));
}
