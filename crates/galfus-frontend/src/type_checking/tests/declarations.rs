use super::*;

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
