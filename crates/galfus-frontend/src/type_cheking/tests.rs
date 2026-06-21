use super::*;

use galfus_core::{DiagnosticCodeKind, SourceId};

use crate::{TypeKind, parse, resolve};

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "test.gfs".to_string(), text.to_string())
}

fn check_source(text: &str) -> (SourceFile, ModuleGraph, TypeCheckResult) {
    let source = source(text);

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

    assert!(!result.has_errors(), "{:?}", result.diagnostics());

    (source, graph, result)
}

fn symbol_by_name_and_kind(graph: &ModuleGraph, name: &str, kind: SymbolKind) -> SymbolId {
    let resolution = graph.resolution().unwrap();

    resolution
        .symbols()
        .iter()
        .find(|symbol| symbol.name() == name && symbol.kind() == kind)
        .map(|symbol| symbol.id())
        .unwrap_or_else(|| panic!("missing symbol `{name}` of kind {kind:?}"))
}

fn find_node_by_kind_and_text(
    source: &SourceFile,
    graph: &ModuleGraph,
    kind: SyntaxNodeKind,
    text: &str,
) -> Option<NodeId> {
    let root = graph.syntax().root()?;
    find_node_by_kind_and_text_from(source, graph, root, kind, text)
}

fn find_node_by_kind_and_text_from(
    source: &SourceFile,
    graph: &ModuleGraph,
    node: NodeId,
    kind: SyntaxNodeKind,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = graph.syntax().node(node)?;

    if syntax_node.kind() == kind && source.slice(syntax_node.span()) == Some(text) {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_node_by_kind_and_text_from(source, graph, *child, kind, text) {
            return Some(found);
        }
    }

    None
}

#[test]
fn check_binds_function_symbol_type() {
    let (_source, graph, result) = check_source(
        r#"
fn main(value: int32): null {
  return
}
"#,
    );

    let main = symbol_by_name_and_kind(&graph, "main", SymbolKind::Function);
    let ty = result.layer().symbol_type(main).unwrap();

    match result.layer().table().kind(ty) {
        Some(TypeKind::Function(function)) => {
            assert_eq!(function.parameters().len(), 1);

            let parameter = function.parameters()[0].ty();
            assert_eq!(
                result.layer().table().kind(parameter),
                Some(&TypeKind::Primitive(PrimitiveType::Int32))
            );

            assert_eq!(
                result.layer().table().kind(function.return_type()),
                Some(&TypeKind::Primitive(PrimitiveType::Null))
            );
        }
        other => panic!("expected function type, got {other:?}"),
    }
}

#[test]
fn check_binds_parameter_symbol_type() {
    let (_source, graph, result) = check_source(
        r#"
fn main(value: int32): null {
  return
}
"#,
    );

    let value = symbol_by_name_and_kind(&graph, "value", SymbolKind::Parameter);
    let ty = result.layer().symbol_type(value).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_binds_rest_parameter_symbol_type() {
    let (_source, graph, result) = check_source(
        r#"
fn main(...values: [int32]): null {
  return
}
"#,
    );

    let values = symbol_by_name_and_kind(&graph, "values", SymbolKind::RestParameter);
    let ty = result.layer().symbol_type(values).unwrap();

    assert!(matches!(
        result.layer().table().kind(ty),
        Some(TypeKind::Array { .. })
    ));
}

#[test]
fn check_binds_struct_field_symbol_type() {
    let (_source, graph, result) = check_source(
        r#"
struct User {
  id: int64,
}
"#,
    );

    let id = symbol_by_name_and_kind(&graph, "id", SymbolKind::StructField);
    let ty = result.layer().symbol_type(id).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int64))
    );
}

#[test]
fn check_binds_var_annotation_symbol_type() {
    let (_source, graph, result) = check_source(
        r#"
var age: int32 = 10
"#,
    );

    let age = symbol_by_name_and_kind(&graph, "age", SymbolKind::Var);
    let ty = result.layer().symbol_type(age).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_binds_const_annotation_symbol_type() {
    let (_source, graph, result) = check_source(
        r#"
const enabled: bool = true
"#,
    );

    let enabled = symbol_by_name_and_kind(&graph, "enabled", SymbolKind::Const);
    let ty = result.layer().symbol_type(enabled).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Bool))
    );
}

#[test]
fn check_binds_type_alias_symbol_type() {
    let (_source, graph, result) = check_source(
        r#"
type MaybeInt = int32 | null
"#,
    );

    let alias = symbol_by_name_and_kind(&graph, "MaybeInt", SymbolKind::TypeAlias);
    let ty = result.layer().symbol_type(alias).unwrap();

    assert!(matches!(
        result.layer().table().kind(ty),
        Some(TypeKind::Union { .. })
    ));
}

#[test]
fn check_binds_named_type_definition_symbol_type() {
    let (_source, graph, result) = check_source(
        r#"
struct User {
  id: int64,
}
"#,
    );

    let user = symbol_by_name_and_kind(&graph, "User", SymbolKind::Struct);
    let ty = result.layer().symbol_type(user).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Named { symbol: user })
    );
}

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
fn check_accepts_name_initializer_with_matching_symbol_type() {
    let (_source, _graph, result) = check_source(
        r#"
var first: int32 = 10
var second: int32 = first
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_empty_return_for_null_function() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_matching_return_type() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): int32 {
  return 10
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_return_type_mismatch() {
    let source = source(
        r#"
fn main(): int32 {
  return true
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
                .contains("expected `int32`, got `bool`")
    }));
}

#[test]
fn check_reports_empty_return_for_non_null_function() {
    let source = source(
        r#"
fn main(): int32 {
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
                .contains("expected `int32`, got `null`")
    }));
}

#[test]
fn check_accepts_nullable_return_type() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): int32 | null {
  return null
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_name_return_with_matching_type() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(value: int32): int32 {
  return value
}
"#,
    );

    assert!(!result.has_errors());
}

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
