use super::*;

#[test]
fn parse_default_parameter() {
    let source = source("fn greet(name: String, punctuation: String = \"!\"): null {\n  return\n}");

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

    let parameter = parameters_node.children()[1];
    let parameter_node = syntax.node(parameter).unwrap();

    assert_eq!(parameter_node.kind(), SyntaxNodeKind::Parameter);
    assert_eq!(
        source.slice(parameter_node.span()),
        Some("punctuation: String = \"!\"")
    );

    assert_eq!(parameter_node.children().len(), 3);

    let default = parameter_node.children()[2];
    let default_node = syntax.node(default).unwrap();

    assert_eq!(default_node.kind(), SyntaxNodeKind::ParameterDefault);
    assert_eq!(source.slice(default_node.span()), Some("= \"!\""));

    let value = default_node.children()[0];

    assert_eq!(
        syntax.node(value).unwrap().kind(),
        SyntaxNodeKind::StringLiteral
    );
}

#[test]
fn parse_multiple_default_parameters() {
    let source =
        source("fn spawn(kind: String = \"enemy\", health: int32 = 100): null {\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.children().len(), 2);

    for parameter in parameters_node.children() {
        let parameter_node = syntax.node(*parameter).unwrap();

        assert_eq!(parameter_node.kind(), SyntaxNodeKind::Parameter);
        assert_eq!(parameter_node.children().len(), 3);

        let default = parameter_node.children()[2];

        assert_eq!(
            syntax.node(default).unwrap().kind(),
            SyntaxNodeKind::ParameterDefault
        );
    }
}

#[test]
fn parse_default_parameter_with_expression() {
    let source = source("fn configure(limit: int32 = 10 + 20): null {\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameter = syntax.node(parameters).unwrap().children()[0];
    let parameter_node = syntax.node(parameter).unwrap();

    let default = parameter_node.children()[2];
    let default_node = syntax.node(default).unwrap();

    let value = default_node.children()[0];

    assert_eq!(
        syntax.node(value).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );

    assert_eq!(
        source.slice(syntax.node(value).unwrap().span()),
        Some("10 + 20")
    );
}

#[test]
fn parse_default_parameter_with_struct_literal() {
    let source = source("fn create(user: User = User { name }): null {\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameter = syntax.node(parameters).unwrap().children()[0];
    let parameter_node = syntax.node(parameter).unwrap();

    let default = parameter_node.children()[2];
    let default_node = syntax.node(default).unwrap();

    let value = default_node.children()[0];
    let value_node = syntax.node(value).unwrap();

    assert_eq!(value_node.kind(), SyntaxNodeKind::StructLiteral);
    assert_eq!(source.slice(value_node.span()), Some("User { name }"));
}

#[test]
fn parse_default_parameter_accepts_trailing_comma() {
    let source = source("fn greet(name: String = \"Ana\",): null {\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.children().len(), 1);

    let parameter = parameters_node.children()[0];
    let parameter_node = syntax.node(parameter).unwrap();

    assert_eq!(parameter_node.kind(), SyntaxNodeKind::Parameter);
    assert_eq!(parameter_node.children().len(), 3);
}

#[test]
fn parse_required_parameter_cannot_follow_default_parameter() {
    let source = source("fn invalid(a: int32 = 1, b: int32): null {\n  return\n}");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result
        .diagnostics()
        .iter()
        .find(|diagnostic| {
            diagnostic.message() == "required parameter cannot follow default parameter"
        })
        .expect("missing required-after-default diagnostic");

    assert_eq!(diagnostic.code().as_str(), "P0006");
}

#[test]
fn parse_rest_parameter_before_default_parameter_is_invalid() {
    let source =
        source("fn invalid(...values: [String], suffix: String = \"!\"): null {\n  return\n}");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message() == "rest parameter must be the last parameter")
        .expect("missing rest-last diagnostic");

    assert_eq!(diagnostic.code().as_str(), "P0006");
}

#[test]
fn parse_rest_parameter_after_default_requires_default() {
    let source =
        source("fn invalid(prefix: String = \"\", ...values: [String]): null {\n  return\n}");

    let result = parse(&source);

    assert!(result.has_errors());

    let diagnostic = result
        .diagnostics()
        .iter()
        .find(|diagnostic| {
            diagnostic.message() == "rest parameter after default parameter must also have default"
        })
        .expect("missing rest-after-default diagnostic");

    assert_eq!(diagnostic.code().as_str(), "P0006");
}

#[test]
fn parse_rest_parameter_after_default_with_default_is_valid() {
    let source = source(
        "fn log(prefix: String = \"\", ...values: [String] | null = null): null {\n  return\n}",
    );

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let parameters = function_node.children()[1];
    let parameters_node = syntax.node(parameters).unwrap();

    assert_eq!(parameters_node.children().len(), 2);

    let rest = parameters_node.children()[1];
    let rest_node = syntax.node(rest).unwrap();

    assert_eq!(rest_node.kind(), SyntaxNodeKind::RestParameter);
    assert_eq!(rest_node.children().len(), 3);

    let rest_type = rest_node.children()[1];
    let rest_default = rest_node.children()[2];

    assert_eq!(
        syntax.node(rest_type).unwrap().kind(),
        SyntaxNodeKind::UnionType
    );

    assert_eq!(
        syntax.node(rest_default).unwrap().kind(),
        SyntaxNodeKind::ParameterDefault
    );
}
