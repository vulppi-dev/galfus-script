use super::*;

#[test]
fn parse_var_statement_with_type_and_initializer() {
    let source = source("fn main(): null { var count: int32 = 1; return }");

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
    let source = source("fn main(): null { const name: String = \"Ana\"; return }");

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
