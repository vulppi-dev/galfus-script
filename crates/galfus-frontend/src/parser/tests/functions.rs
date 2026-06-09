use super::*;

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
