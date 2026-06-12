use crate::RangeOperatorKind;

use super::*;

#[test]
fn parse_exclusive_range_expression() {
    let source = source("var range = 1..9");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let var_item = syntax.first_child(root).unwrap();

    let initializer = syntax
        .first_child_of_kind(var_item, SyntaxNodeKind::Initializer)
        .unwrap();

    let expression = syntax.first_child(initializer).unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::RangeExpression);
    assert_eq!(expression_node.child_count(), 3);

    let operator = syntax.child(expression, 1).unwrap();
    let operator_node = syntax.node(operator).unwrap();

    assert_eq!(operator_node.kind(), SyntaxNodeKind::RangeOperator);
    assert_eq!(
        operator_node.range_operator(),
        Some(RangeOperatorKind::Exclusive)
    );
}

#[test]
fn parse_quantity_range_expression() {
    let source = source("var range = 1::4");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let var_item = syntax.first_child(root).unwrap();

    let initializer = syntax
        .first_child_of_kind(var_item, SyntaxNodeKind::Initializer)
        .unwrap();

    let expression = syntax.first_child(initializer).unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::RangeExpression);
    assert_eq!(expression_node.child_count(), 3);

    let operator = syntax.child(expression, 1).unwrap();
    let operator_node = syntax.node(operator).unwrap();

    assert_eq!(
        operator_node.range_operator(),
        Some(RangeOperatorKind::Quantity)
    );
}

#[test]
fn parse_quantity_range_expression_with_step() {
    let source = source("var range = 1::4%3");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let var_item = syntax.first_child(root).unwrap();

    let initializer = syntax
        .first_child_of_kind(var_item, SyntaxNodeKind::Initializer)
        .unwrap();

    let expression = syntax.first_child(initializer).unwrap();
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::RangeExpression);
    assert_eq!(expression_node.child_count(), 4);

    let step = syntax.child(expression, 3).unwrap();
    let step_node = syntax.node(step).unwrap();

    assert_eq!(step_node.kind(), SyntaxNodeKind::RangeStep);

    let step_value = syntax.first_child(step).unwrap();

    assert_eq!(
        syntax.node(step_value).unwrap().kind(),
        SyntaxNodeKind::IntegerLiteral
    );
}

#[test]
fn parse_namespace_call_still_uses_path_expression() {
    let source = source("var value = math::random()");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let var_item = syntax.first_child(root).unwrap();

    let initializer = syntax
        .first_child_of_kind(var_item, SyntaxNodeKind::Initializer)
        .unwrap();

    let call = syntax.first_child(initializer).unwrap();
    let call_node = syntax.node(call).unwrap();

    assert_eq!(call_node.kind(), SyntaxNodeKind::CallExpression);

    let callee = syntax.child(call, 0).unwrap();

    assert_eq!(
        syntax.node(callee).unwrap().kind(),
        SyntaxNodeKind::PathExpression
    );
}

#[test]
fn parse_identifier_colon_colon_identifier_as_path_expression() {
    let source = source("var value = a::b");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();
    let root = syntax.root().unwrap();
    let var_item = syntax.first_child(root).unwrap();

    let initializer = syntax
        .first_child_of_kind(var_item, SyntaxNodeKind::Initializer)
        .unwrap();

    let expression = syntax.first_child(initializer).unwrap();

    assert_eq!(
        syntax.node(expression).unwrap().kind(),
        SyntaxNodeKind::PathExpression
    );
}
