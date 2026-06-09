use super::*;

#[test]
fn parse_struct_item() {
    let source = source("struct User { name: String, age: int32 }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.children().len(), 1);

    let item = root_node.children()[0];
    let item_node = syntax.node(item).unwrap();

    assert_eq!(item_node.kind(), SyntaxNodeKind::StructItem);
    assert_eq!(item_node.children().len(), 2);

    let name = item_node.children()[0];
    let fields = item_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("User")
    );

    let fields_node = syntax.node(fields).unwrap();

    assert_eq!(fields_node.kind(), SyntaxNodeKind::StructFieldList);
    assert_eq!(fields_node.children().len(), 2);

    let first_field = fields_node.children()[0];
    let first_field_node = syntax.node(first_field).unwrap();

    assert_eq!(first_field_node.kind(), SyntaxNodeKind::StructField);
    assert_eq!(source.slice(first_field_node.span()), Some("name: String"));

    let first_field_name = first_field_node.children()[0];
    let first_field_type = first_field_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(first_field_name).unwrap().span()),
        Some("name")
    );

    assert_eq!(
        syntax.node(first_field_type).unwrap().kind(),
        SyntaxNodeKind::TypeName
    );

    assert_eq!(
        source.slice(syntax.node(first_field_type).unwrap().span()),
        Some("String")
    );
}

#[test]
fn parse_struct_fields_with_commas() {
    let source = source("struct User { name: String, age: int32, }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let item = syntax.node(root).unwrap().children()[0];
    let item_node = syntax.node(item).unwrap();

    let fields = item_node.children()[1];
    let fields_node = syntax.node(fields).unwrap();

    assert_eq!(fields_node.kind(), SyntaxNodeKind::StructFieldList);
    assert_eq!(fields_node.children().len(), 2);

    let second_field = fields_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(second_field).unwrap().span()),
        Some("age: int32")
    );
}

#[test]
fn parse_struct_followed_by_function() {
    let source = source("struct User { name: String }\nfn main(): null { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.children().len(), 2);

    let struct_item = root_node.children()[0];
    let function_item = root_node.children()[1];

    assert_eq!(
        syntax.node(struct_item).unwrap().kind(),
        SyntaxNodeKind::StructItem
    );

    assert_eq!(
        syntax.node(function_item).unwrap().kind(),
        SyntaxNodeKind::FunctionItem
    );
}

#[test]
fn parse_export_struct_item() {
    let source = source("export struct User { name: String }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.children().len(), 1);

    let export = root_node.children()[0];
    let export_node = syntax.node(export).unwrap();

    assert_eq!(export_node.kind(), SyntaxNodeKind::ExportItem);
    assert_eq!(
        source.slice(export_node.span()),
        Some("export struct User { name: String }")
    );
    assert_eq!(export_node.children().len(), 1);

    let inner = export_node.children()[0];
    let inner_node = syntax.node(inner).unwrap();

    assert_eq!(inner_node.kind(), SyntaxNodeKind::StructItem);

    let name = inner_node.children()[0];

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("User")
    );
}

#[test]
fn parse_struct_requires_commas_between_fields() {
    let source = source("struct User { name: String age: int32 }");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "P0001");
    assert_eq!(diagnostic.message(), "expected `Comma`, found `Identifier`");
}

#[test]
fn parse_enum_requires_commas_between_variants() {
    let source = source("enum Direction { North South }");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "P0001");
    assert_eq!(diagnostic.message(), "expected `Comma`, found `Identifier`");
}

#[test]
fn parse_choice_item_with_payload_variants() {
    let source = source("choice Result { Ok(User), SomeError(int32, String), }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let item = syntax.node(root).unwrap().children()[0];
    let item_node = syntax.node(item).unwrap();

    assert_eq!(item_node.kind(), SyntaxNodeKind::ChoiceItem);
    assert_eq!(item_node.children().len(), 2);

    let name = item_node.children()[0];
    let variants = item_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("Result")
    );

    let variants_node = syntax.node(variants).unwrap();

    assert_eq!(variants_node.kind(), SyntaxNodeKind::ChoiceVariantList);
    assert_eq!(variants_node.children().len(), 2);

    let ok_variant = variants_node.children()[0];
    let ok_variant_node = syntax.node(ok_variant).unwrap();

    assert_eq!(ok_variant_node.kind(), SyntaxNodeKind::ChoiceVariant);
    assert_eq!(source.slice(ok_variant_node.span()), Some("Ok(User)"));
    assert_eq!(ok_variant_node.children().len(), 2);

    let payload = ok_variant_node.children()[1];
    let payload_node = syntax.node(payload).unwrap();

    assert_eq!(payload_node.kind(), SyntaxNodeKind::ChoicePayload);
    assert_eq!(payload_node.children().len(), 1);

    let payload_type = payload_node.children()[0];

    assert_eq!(
        syntax.node(payload_type).unwrap().kind(),
        SyntaxNodeKind::TypeName
    );

    assert_eq!(
        source.slice(syntax.node(payload_type).unwrap().span()),
        Some("User")
    );
}

#[test]
fn parse_choice_variant_with_multiple_payload_types() {
    let source = source("choice Result { SomeError(int32, String), }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let item = syntax.node(root).unwrap().children()[0];
    let item_node = syntax.node(item).unwrap();

    let variants = item_node.children()[1];
    let variant = syntax.node(variants).unwrap().children()[0];
    let variant_node = syntax.node(variant).unwrap();

    assert_eq!(
        source.slice(variant_node.span()),
        Some("SomeError(int32, String)")
    );

    let payload = variant_node.children()[1];
    let payload_node = syntax.node(payload).unwrap();

    assert_eq!(payload_node.kind(), SyntaxNodeKind::ChoicePayload);
    assert_eq!(payload_node.children().len(), 2);

    let first_type = payload_node.children()[0];
    let second_type = payload_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(first_type).unwrap().span()),
        Some("int32")
    );

    assert_eq!(
        source.slice(syntax.node(second_type).unwrap().span()),
        Some("String")
    );
}

#[test]
fn parse_choice_item_with_unit_variant() {
    let source = source("choice Option { Some(User), None, }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let item = syntax.node(root).unwrap().children()[0];
    let item_node = syntax.node(item).unwrap();

    let variants = item_node.children()[1];
    let variants_node = syntax.node(variants).unwrap();

    let none_variant = variants_node.children()[1];
    let none_variant_node = syntax.node(none_variant).unwrap();

    assert_eq!(none_variant_node.kind(), SyntaxNodeKind::ChoiceVariant);
    assert_eq!(source.slice(none_variant_node.span()), Some("None"));
    assert_eq!(none_variant_node.children().len(), 1);
}
