use super::*;

#[test]
fn parse_generic_anchor_call_expression() {
    let source =
        source("fn main(): null {\n  var buffer = buffer::array<int32>(size)\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let statement_node = syntax.node(statement).unwrap();

    let initializer = statement_node.children()[1];
    let expression = syntax.node(initializer).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CallExpression);
    assert_eq!(
        source.slice(expression_node.span()),
        Some("buffer::array<int32>(size)")
    );

    let target = expression_node.children()[0];
    let target_node = syntax.node(target).unwrap();

    assert_eq!(target_node.kind(), SyntaxNodeKind::GenericExpression);

    let generic_target = target_node.children()[0];
    let generic_args = target_node.children()[1];

    assert_eq!(
        syntax.node(generic_target).unwrap().kind(),
        SyntaxNodeKind::AnchorExpression
    );

    assert_eq!(
        syntax.node(generic_args).unwrap().kind(),
        SyntaxNodeKind::GenericArgumentList
    );
}

#[test]
fn parse_generic_member_call_expression() {
    let source = source("fn main(): null {\n  registry.get<String>(\"name\")\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let expression = syntax.node(statement).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CallExpression);

    let target = expression_node.children()[0];
    let target_node = syntax.node(target).unwrap();

    assert_eq!(target_node.kind(), SyntaxNodeKind::GenericExpression);
    assert_eq!(
        source.slice(target_node.span()),
        Some("registry.get<String>")
    );
}

#[test]
fn parse_generic_call_with_multiple_arguments() {
    let source = source("fn main(): null {\n  makePair<String, User>(name, user)\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let expression = syntax.node(statement).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    let target = expression_node.children()[0];
    let target_node = syntax.node(target).unwrap();

    let generic_args = target_node.children()[1];
    let generic_args_node = syntax.node(generic_args).unwrap();

    assert_eq!(generic_args_node.children().len(), 2);
    assert_eq!(
        source.slice(generic_args_node.span()),
        Some("<String, User>")
    );
}

#[test]
fn parse_generic_call_with_nested_generic_argument() {
    let source = source("fn main(): null {\n  makeBox<Map<String, User>>(value)\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let expression = syntax.node(statement).unwrap().children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CallExpression);
    assert_eq!(
        source.slice(expression_node.span()),
        Some("makeBox<Map<String, User>>(value)")
    );

    let target = expression_node.children()[0];
    let generic_args = syntax.node(target).unwrap().children()[1];
    let first_arg = syntax.node(generic_args).unwrap().children()[0];

    assert_eq!(
        syntax.node(first_arg).unwrap().kind(),
        SyntaxNodeKind::GenericType
    );
}

#[test]
fn parse_less_than_expression_still_works() {
    let source = source("fn main(): null {\n  const ok = a < b\n  return\n}");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().children()[0];
    let function_node = syntax.node(function).unwrap();

    let body = function_node.children()[3];
    let statement = syntax.node(body).unwrap().children()[0];
    let initializer = syntax.node(statement).unwrap().children()[1];
    let expression = syntax.node(initializer).unwrap().children()[0];

    assert_eq!(
        syntax.node(expression).unwrap().kind(),
        SyntaxNodeKind::BinaryExpression
    );
}

#[test]
fn parse_generic_expression_without_call_is_not_generic_expression() {
    let source = source("fn main(): null {\n  const ok = makeBox<User>\n  return\n}");

    let result = parse(&source);

    assert!(result.has_errors());
}
