use super::*;
use galfus_core::{SourceFile, SourceId};

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "test.gfs".to_string(), text.to_string())
}

#[test]
fn parse_stores_tokens_in_graph() {
    let source = source("fn main(): null { return }");

    let result = parse(&source);

    assert!(!result.graph().syntax().tokens().is_empty());
}

#[test]
fn parser_starts_at_first_token() {
    let source = source("fn main(): null { return }");
    let lex_result = lex(&source);
    let (tokens, diagnostics) = lex_result.into_parts();

    let parser = Parser::new(&source, tokens, diagnostics);

    assert_eq!(parser.current().kind(), &TokenKind::Fn);
}

#[test]
fn parser_bump_consumes_current_token() {
    let source = source("fn main(): null { return }");
    let lex_result = lex(&source);
    let (tokens, diagnostics) = lex_result.into_parts();

    let mut parser = Parser::new(&source, tokens, diagnostics);

    let token = parser.bump();

    assert_eq!(token.kind(), &TokenKind::Fn);
    assert_eq!(parser.current().kind(), &TokenKind::Identifier);
}

#[test]
fn parser_expect_consumes_expected_token() {
    let source = source("fn main(): null { return }");
    let lex_result = lex(&source);
    let (tokens, diagnostics) = lex_result.into_parts();

    let mut parser = Parser::new(&source, tokens, diagnostics);

    let token = parser.expect(TokenKind::Fn);

    assert!(token.is_some());
    assert_eq!(parser.current().kind(), &TokenKind::Identifier);
}

#[test]
fn parser_expect_reports_unexpected_token() {
    let source = source("return");
    let lex_result = lex(&source);
    let (tokens, diagnostics) = lex_result.into_parts();

    let mut parser = Parser::new(&source, tokens, diagnostics);

    let token = parser.expect(TokenKind::Fn);

    assert!(token.is_none());
    assert!(parser.graph.has_errors());

    let diagnostic = parser.graph.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "P0001");
    assert_eq!(diagnostic.message(), "expected `Fn`, found `Return`");
}

#[test]
fn parse_creates_source_file_root() {
    let source = source("fn main(): null { return }");

    let result = parse(&source);

    let syntax = result.graph().syntax();
    let root = syntax.root().expect("parse should create root node");
    let root_node = syntax.node(root).expect("root node should exist");

    assert_eq!(root_node.kind(), SyntaxNodeKind::SourceFile);
    assert_eq!(
        source.slice(root_node.span()),
        Some("fn main(): null { return }")
    );
}

#[test]
fn parse_creates_root_even_with_lexical_diagnostics() {
    let source = source("\"unterminated");

    let result = parse(&source);

    assert!(result.has_errors());
    assert!(result.graph().syntax().root().is_some());

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "L0002");
}

#[test]
fn parse_function_item_minimal() {
    let source = source("fn main(): null {}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.kind(), SyntaxNodeKind::SourceFile);
    assert_eq!(root_node.children().len(), 1);

    let function = root_node.children()[0];
    let function_node = syntax.node(function).unwrap();

    assert_eq!(function_node.kind(), SyntaxNodeKind::FunctionItem);
    assert_eq!(
        source.slice(function_node.span()),
        Some("fn main(): null {}")
    );
    assert_eq!(function_node.children().len(), 4);

    let name = function_node.children()[0];
    let parameters = function_node.children()[1];
    let return_type = function_node.children()[2];
    let body = function_node.children()[3];

    assert_eq!(
        syntax.node(name).unwrap().kind(),
        SyntaxNodeKind::Identifier
    );
    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("main")
    );

    assert_eq!(
        syntax.node(parameters).unwrap().kind(),
        SyntaxNodeKind::ParameterList
    );

    assert_eq!(
        syntax.node(return_type).unwrap().kind(),
        SyntaxNodeKind::TypeNull
    );

    assert_eq!(syntax.node(body).unwrap().kind(), SyntaxNodeKind::Block);
}

#[test]
fn parse_reports_expected_item_at_top_level() {
    let source = source("return");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "P0002");
    assert_eq!(diagnostic.message(), "expected item, found `Return`");
}

#[test]
fn parse_return_statement_inside_block() {
    let source = source("fn main(): null { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    let function = root_node.children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    assert_eq!(body_node.kind(), SyntaxNodeKind::Block);
    assert_eq!(body_node.children().len(), 1);

    let statement = body_node.children()[0];
    let statement_node = syntax.node(statement).unwrap();

    assert_eq!(statement_node.kind(), SyntaxNodeKind::ReturnStatement);
    assert_eq!(source.slice(statement_node.span()), Some("return"));
}

#[test]
fn parse_reports_expected_statement_inside_block() {
    let source = source("fn main(): null { fn }");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "P0005");
    assert_eq!(diagnostic.message(), "expected statement, found `Fn`");
}

#[test]
fn parse_function_parameters() {
    let source = source("fn sum(a: int32, b: int32): int32 { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    let function = root_node.children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.kind(), SyntaxNodeKind::ParameterList);
    assert_eq!(parameters_node.children().len(), 2);

    let first_parameter = parameters_node.children()[0];
    let first_parameter_node = syntax.node(first_parameter).unwrap();

    assert_eq!(first_parameter_node.kind(), SyntaxNodeKind::Parameter);
    assert_eq!(source.slice(first_parameter_node.span()), Some("a: int32"));

    let first_name = first_parameter_node.children()[0];
    let first_type = first_parameter_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(first_name).unwrap().span()),
        Some("a")
    );
    assert_eq!(
        syntax.node(first_type).unwrap().kind(),
        SyntaxNodeKind::TypeName
    );
    assert_eq!(
        source.slice(syntax.node(first_type).unwrap().span()),
        Some("int32")
    );

    let second_parameter = parameters_node.children()[1];
    let second_parameter_node = syntax.node(second_parameter).unwrap();

    assert_eq!(source.slice(second_parameter_node.span()), Some("b: int32"));
}

#[test]
fn parse_empty_parameter_list_still_works() {
    let source = source("fn main(): null { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.kind(), SyntaxNodeKind::ParameterList);
    assert!(parameters_node.children().is_empty());
}

#[test]
fn parse_parameter_list_accepts_trailing_comma() {
    let source = source("fn sum(a: int32, b: int32,): int32 { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.kind(), SyntaxNodeKind::ParameterList);
    assert_eq!(parameters_node.children().len(), 2);
}

#[test]
fn parse_parameter_list_accepts_multiline_trailing_comma() {
    let source = source("fn sum(\n  a: int32,\n  b: int32,\n): int32 {\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.kind(), SyntaxNodeKind::ParameterList);
    assert_eq!(parameters_node.children().len(), 2);
}

#[test]
fn parse_array_type_in_parameter() {
    let source = source("fn first(values: [int32]): int32 { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameters_node = syntax.node(parameters).unwrap();

    let parameter = parameters_node.children()[0];
    let parameter_node = syntax.node(parameter).unwrap();

    let parameter_type = parameter_node.children()[1];
    let parameter_type_node = syntax.node(parameter_type).unwrap();

    assert_eq!(parameter_type_node.kind(), SyntaxNodeKind::ArrayType);
    assert_eq!(source.slice(parameter_type_node.span()), Some("[int32]"));

    let element_type = parameter_type_node.children()[0];
    let element_type_node = syntax.node(element_type).unwrap();

    assert_eq!(element_type_node.kind(), SyntaxNodeKind::TypeName);
    assert_eq!(source.slice(element_type_node.span()), Some("int32"));
}

#[test]
fn parse_nested_array_type() {
    let source = source("fn matrix(values: [[int32]]): null { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameter = syntax.node(parameters).unwrap().children()[0];
    let parameter_node = syntax.node(parameter).unwrap();

    let outer_array = parameter_node.children()[1];
    let outer_array_node = syntax.node(outer_array).unwrap();

    assert_eq!(outer_array_node.kind(), SyntaxNodeKind::ArrayType);
    assert_eq!(source.slice(outer_array_node.span()), Some("[[int32]]"));

    let inner_array = outer_array_node.children()[0];
    let inner_array_node = syntax.node(inner_array).unwrap();

    assert_eq!(inner_array_node.kind(), SyntaxNodeKind::ArrayType);
    assert_eq!(source.slice(inner_array_node.span()), Some("[int32]"));
}

#[test]
fn parse_fixed_array_type_with_integer_size() {
    let source = source("fn take(values: [int32; 3]): null { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameter = syntax.node(parameters).unwrap().children()[0];
    let parameter_node = syntax.node(parameter).unwrap();

    let parameter_type = parameter_node.children()[1];
    let parameter_type_node = syntax.node(parameter_type).unwrap();

    assert_eq!(parameter_type_node.kind(), SyntaxNodeKind::FixedArrayType);
    assert_eq!(source.slice(parameter_type_node.span()), Some("[int32; 3]"));

    let size = parameter_type_node.children()[1];
    let size_node = syntax.node(size).unwrap();

    assert_eq!(size_node.kind(), SyntaxNodeKind::ArraySize);
    assert_eq!(source.slice(size_node.span()), Some("3"));

    let size_value = size_node.children()[0];

    assert_eq!(
        syntax.node(size_value).unwrap().kind(),
        SyntaxNodeKind::IntegerLiteral
    );
}

#[test]
fn parse_reports_named_fixed_array_size_as_error() {
    let source = source("fn take(values: [int32; n]): null { return }");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "P0001");
    assert_eq!(
        diagnostic.message(),
        "expected array size integer literal, found `Identifier`"
    );
}

#[test]
fn parse_union_return_type() {
    let source = source("fn find(): User | null { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let return_type = function_node.children()[2];
    let return_type_node = syntax.node(return_type).unwrap();

    assert_eq!(return_type_node.kind(), SyntaxNodeKind::UnionType);
    assert_eq!(source.slice(return_type_node.span()), Some("User | null"));
    assert_eq!(return_type_node.children().len(), 2);

    let first = return_type_node.children()[0];
    let second = return_type_node.children()[1];

    assert_eq!(syntax.node(first).unwrap().kind(), SyntaxNodeKind::TypeName);
    assert_eq!(
        source.slice(syntax.node(first).unwrap().span()),
        Some("User")
    );

    assert_eq!(
        syntax.node(second).unwrap().kind(),
        SyntaxNodeKind::TypeNull
    );
    assert_eq!(
        source.slice(syntax.node(second).unwrap().span()),
        Some("null")
    );
}

#[test]
fn parse_union_type_inside_array() {
    let source = source("fn many(values: [User | null]): null { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameter = syntax.node(parameters).unwrap().children()[0];
    let parameter_node = syntax.node(parameter).unwrap();

    let parameter_type = parameter_node.children()[1];
    let array_node = syntax.node(parameter_type).unwrap();

    assert_eq!(array_node.kind(), SyntaxNodeKind::ArrayType);
    assert_eq!(source.slice(array_node.span()), Some("[User | null]"));

    let element_type = array_node.children()[0];
    let element_type_node = syntax.node(element_type).unwrap();

    assert_eq!(element_type_node.kind(), SyntaxNodeKind::UnionType);
    assert_eq!(source.slice(element_type_node.span()), Some("User | null"));
}

#[test]
fn parse_type_alias_item() {
    let source = source("type UserId = int32");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.children().len(), 1);

    let item = root_node.children()[0];
    let item_node = syntax.node(item).unwrap();

    assert_eq!(item_node.kind(), SyntaxNodeKind::TypeAliasItem);
    assert_eq!(source.slice(item_node.span()), Some("type UserId = int32"));
    assert_eq!(item_node.children().len(), 2);

    let name = item_node.children()[0];
    let aliased_type = item_node.children()[1];

    assert_eq!(
        syntax.node(name).unwrap().kind(),
        SyntaxNodeKind::Identifier
    );
    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("UserId")
    );

    assert_eq!(
        syntax.node(aliased_type).unwrap().kind(),
        SyntaxNodeKind::TypeName
    );
    assert_eq!(
        source.slice(syntax.node(aliased_type).unwrap().span()),
        Some("int32")
    );
}

#[test]
fn parse_type_alias_followed_by_function() {
    let source = source("type UserId = int32\nfn main(): null { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let root_node = syntax.node(root).unwrap();

    assert_eq!(root_node.children().len(), 2);

    let alias = root_node.children()[0];
    let function = root_node.children()[1];

    assert_eq!(
        syntax.node(alias).unwrap().kind(),
        SyntaxNodeKind::TypeAliasItem
    );

    assert_eq!(
        syntax.node(function).unwrap().kind(),
        SyntaxNodeKind::FunctionItem
    );
}

#[test]
fn parse_export_type_alias_item() {
    let source = source("export type UserId = int32");

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
        Some("export type UserId = int32")
    );
    assert_eq!(export_node.children().len(), 1);

    let inner = export_node.children()[0];
    let inner_node = syntax.node(inner).unwrap();

    assert_eq!(inner_node.kind(), SyntaxNodeKind::TypeAliasItem);
}

#[test]
fn parse_export_function_item() {
    let source = source("export fn main(): null { return }");

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
        Some("export fn main(): null { return }")
    );
    assert_eq!(export_node.children().len(), 1);

    let inner = export_node.children()[0];
    let inner_node = syntax.node(inner).unwrap();

    assert_eq!(inner_node.kind(), SyntaxNodeKind::FunctionItem);
}

#[test]
fn parse_reports_invalid_export_item() {
    let source = source("export return");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "P0002");
    assert_eq!(
        diagnostic.message(),
        "expected exportable item, found `Return`"
    );
}
#[test]
fn parse_namespace_import_item() {
    let source = source("import math from \"std/math\"");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let import = syntax.node(root).unwrap().children()[0];
    let import_node = syntax.node(import).unwrap();

    let clause = import_node.children()[0];
    let clause_node = syntax.node(clause).unwrap();

    assert_eq!(clause_node.kind(), SyntaxNodeKind::NamespaceImport);

    let name = clause_node.children()[0];

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("math")
    );
}

#[test]
fn parse_named_import_list() {
    let source = source("import { sin, cos } from \"std/math\"");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let import = syntax.node(root).unwrap().children()[0];
    let import_node = syntax.node(import).unwrap();

    let clause = import_node.children()[0];
    let clause_node = syntax.node(clause).unwrap();

    assert_eq!(clause_node.kind(), SyntaxNodeKind::NamedImportList);
    assert_eq!(source.slice(clause_node.span()), Some("{ sin, cos }"));
    assert_eq!(clause_node.children().len(), 2);

    let first = clause_node.children()[0];
    let second = clause_node.children()[1];

    assert_eq!(
        syntax.node(first).unwrap().kind(),
        SyntaxNodeKind::NamedImport
    );
    assert_eq!(
        syntax.node(second).unwrap().kind(),
        SyntaxNodeKind::NamedImport
    );
}

#[test]
fn parse_named_import_alias() {
    let source = source("import { sin as sine, cos } from \"std/math\"");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let import = syntax.node(root).unwrap().children()[0];
    let import_node = syntax.node(import).unwrap();

    let clause = import_node.children()[0];
    let clause_node = syntax.node(clause).unwrap();

    assert_eq!(clause_node.kind(), SyntaxNodeKind::NamedImportList);
    assert_eq!(clause_node.children().len(), 2);

    let first_import = clause_node.children()[0];
    let first_import_node = syntax.node(first_import).unwrap();

    assert_eq!(first_import_node.kind(), SyntaxNodeKind::NamedImport);
    assert_eq!(source.slice(first_import_node.span()), Some("sin as sine"));
    assert_eq!(first_import_node.children().len(), 2);

    let imported_name = first_import_node.children()[0];
    let alias = first_import_node.children()[1];

    assert_eq!(
        source.slice(syntax.node(imported_name).unwrap().span()),
        Some("sin")
    );

    let alias_node = syntax.node(alias).unwrap();

    assert_eq!(alias_node.kind(), SyntaxNodeKind::ImportAlias);
    assert_eq!(source.slice(alias_node.span()), Some("as sine"));

    let local_name = alias_node.children()[0];

    assert_eq!(
        source.slice(syntax.node(local_name).unwrap().span()),
        Some("sine")
    );

    let second_import = clause_node.children()[1];
    let second_import_node = syntax.node(second_import).unwrap();

    assert_eq!(source.slice(second_import_node.span()), Some("cos"));
    assert_eq!(second_import_node.children().len(), 1);
}

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

#[test]
fn parse_var_statement_with_type_and_initializer() {
    let source = source("fn main(): null { var count: int32 = 1 return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let var_statement = body_node.children()[0];
    let var_node = syntax.node(var_statement).unwrap();

    assert_eq!(var_node.kind(), SyntaxNodeKind::VarStatement);
    assert_eq!(source.slice(var_node.span()), Some("var count: int32 = 1"));
    assert_eq!(var_node.children().len(), 3);

    let name = var_node.children()[0];
    let annotation = var_node.children()[1];
    let initializer = var_node.children()[2];

    assert_eq!(
        source.slice(syntax.node(name).unwrap().span()),
        Some("count")
    );
    assert_eq!(
        syntax.node(annotation).unwrap().kind(),
        SyntaxNodeKind::TypeAnnotation
    );
    assert_eq!(
        syntax.node(initializer).unwrap().kind(),
        SyntaxNodeKind::Initializer
    );
}

#[test]
fn parse_const_statement_with_string_initializer() {
    let source = source("fn main(): null { const name: String = \"Ana\" return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let const_statement = body_node.children()[0];
    let const_node = syntax.node(const_statement).unwrap();

    assert_eq!(const_node.kind(), SyntaxNodeKind::ConstStatement);
    assert_eq!(
        source.slice(const_node.span()),
        Some("const name: String = \"Ana\"")
    );

    let initializer = const_node.children()[2];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];

    assert_eq!(
        syntax.node(expression).unwrap().kind(),
        SyntaxNodeKind::StringLiteral
    );
}

#[test]
fn parse_const_requires_initializer() {
    let source = source("fn main(): null { const name: String return }");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "P0001");
    assert_eq!(diagnostic.message(), "expected `Equal`, found `Return`");
}

#[test]
fn parse_return_statement_with_integer_expression() {
    let source = source("fn one(): int32 { return 1 }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    assert_eq!(return_node.kind(), SyntaxNodeKind::ReturnStatement);
    assert_eq!(source.slice(return_node.span()), Some("return 1"));
    assert_eq!(return_node.children().len(), 1);

    let expression = return_node.children()[0];

    assert_eq!(
        syntax.node(expression).unwrap().kind(),
        SyntaxNodeKind::IntegerLiteral
    );

    assert_eq!(
        source.slice(syntax.node(expression).unwrap().span()),
        Some("1")
    );
}

#[test]
fn parse_return_statement_with_null_expression() {
    let source = source("fn none(): null { return null }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    assert_eq!(return_node.kind(), SyntaxNodeKind::ReturnStatement);
    assert_eq!(source.slice(return_node.span()), Some("return null"));
    assert_eq!(return_node.children().len(), 1);

    let expression = return_node.children()[0];

    assert_eq!(
        syntax.node(expression).unwrap().kind(),
        SyntaxNodeKind::NullLiteral
    );
}

#[test]
fn parse_empty_return_statement_still_works() {
    let source = source("fn main(): null { return }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let body_node = syntax.node(body).unwrap();

    let return_statement = body_node.children()[0];
    let return_node = syntax.node(return_statement).unwrap();

    assert_eq!(return_node.kind(), SyntaxNodeKind::ReturnStatement);
    assert_eq!(source.slice(return_node.span()), Some("return"));
    assert!(return_node.children().is_empty());
}
