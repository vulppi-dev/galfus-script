use super::*;

#[test]
fn check_accepts_enum_variant_expression() {
    let (_source, _graph, result) = check_source(
        r#"
enum Direction {
  North,
  South,
}

var direction: Direction = Direction::North
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_binds_enum_variant_expression_type() {
    let (source, graph, result) = check_source(
        r#"
enum Direction {
  North,
  South,
}

var direction: Direction = Direction::North
"#,
    );

    let path = find_node_by_kind_and_text(
        &source,
        &graph,
        SyntaxNodeKind::PathExpression,
        "Direction::North",
    )
    .unwrap();

    let direction_symbol = symbol_by_name_and_kind(&graph, "Direction", SymbolKind::Enum);
    let ty = result.layer().node_type(path).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Named {
            symbol: direction_symbol
        })
    );
}

#[test]
fn check_accepts_integer_enum_base_type() {
    let (_source, _graph, result) = check_source(
        r#"
enum(uint8) Mode {
  Off(0),
  On(1),
}

var mode: Mode = Mode::On
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_non_integer_enum_base_type() {
    let source = source(
        r#"
enum(bool) Mode {
  Off,
  On,
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
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidEnumBaseType.as_code()
            && diagnostic
                .message()
                .contains("enum base type must be an integer")
    }));
}

#[test]
fn check_reports_enum_discriminant_type_mismatch() {
    let source = source(
        r#"
enum(uint8) Mode {
  Off(true),
  On(1),
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
                .contains("expected `uint8`, got `bool`")
    }));
}

#[test]
fn check_accepts_choice_variant_without_payload() {
    let (_source, _graph, result) = check_source(
        r#"
choice Asset {
  None,
  Texture([uint8]),
}

var asset: Asset = Asset::None
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_choice_variant_with_payload() {
    let (_source, _graph, result) = check_source(
        r#"
choice Asset {
  None,
  Texture([uint8]),
}

var asset: Asset = Asset::Texture("grass.png")
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_choice_variant_with_multiple_payload_items() {
    let (_source, _graph, result) = check_source(
        r#"
choice Asset {
  Image([uint8], int32, int32),
}

var asset: Asset = Asset::Image("grass.png", 64, 64)
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_choice_payload_required() {
    let source = source(
        r#"
choice Asset {
  Texture([uint8]),
}

var asset: Asset = Asset::Texture
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
        diagnostic.code().as_str() == TypeDiagnosticCode::ChoicePayloadRequired.as_code()
            && diagnostic.message().contains("requires a payload")
    }));
}

#[test]
fn check_reports_choice_payload_not_allowed() {
    let source = source(
        r#"
choice Asset {
  None,
}

var asset: Asset = Asset::None(1)
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
        diagnostic.code().as_str() == TypeDiagnosticCode::ChoicePayloadNotAllowed.as_code()
            && diagnostic.message().contains("does not accept a payload")
    }));
}

#[test]
fn check_reports_choice_payload_argument_count_mismatch() {
    let source = source(
        r#"
choice Asset {
  Image([uint8], int32, int32),
}

var asset: Asset = Asset::Image("grass.png", 64)
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
            && diagnostic.message().contains("expected 3 arguments, got 2")
    }));
}

#[test]
fn check_reports_choice_payload_type_mismatch() {
    let source = source(
        r#"
choice Asset {
  Image([uint8], int32, int32),
}

var asset: Asset = Asset::Image("grass.png", true, 64)
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
