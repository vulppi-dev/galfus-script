use super::*;

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

    assert_eq!(element_type_node.kind(), SyntaxNodeKind::NamedType);
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

    assert_eq!(
        syntax.node(first).unwrap().kind(),
        SyntaxNodeKind::NamedType
    );
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
fn parse_qualified_type_as_path() {
    let source = source("fn main(value: collections::List): null { return }");

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

    assert_eq!(parameter_type_node.kind(), SyntaxNodeKind::Path);
    assert_eq!(
        source.slice(parameter_type_node.span()),
        Some("collections::List")
    );
    assert_eq!(parameter_type_node.children().len(), 2);

    let first = parameter_type_node.children()[0];
    let second = parameter_type_node.children()[1];

    assert_eq!(
        syntax.node(first).unwrap().kind(),
        SyntaxNodeKind::Identifier
    );
    assert_eq!(
        source.slice(syntax.node(first).unwrap().span()),
        Some("collections")
    );

    assert_eq!(
        syntax.node(second).unwrap().kind(),
        SyntaxNodeKind::Identifier
    );
    assert_eq!(
        source.slice(syntax.node(second).unwrap().span()),
        Some("List")
    );
}

#[test]
fn parse_qualified_generic_type_uses_path_base() {
    let source = source("fn main(value: collections::List<int32>): null { return }");

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

    assert_eq!(parameter_type_node.kind(), SyntaxNodeKind::GenericType);
    assert_eq!(
        source.slice(parameter_type_node.span()),
        Some("collections::List<int32>")
    );

    let base = parameter_type_node.children()[0];
    let base_node = syntax.node(base).unwrap();

    assert_eq!(base_node.kind(), SyntaxNodeKind::Path);
    assert_eq!(source.slice(base_node.span()), Some("collections::List"));

    let arguments = parameter_type_node.children()[1];
    let arguments_node = syntax.node(arguments).unwrap();

    assert_eq!(arguments_node.kind(), SyntaxNodeKind::TypeArgumentList);
}

#[test]
fn parse_namespace_like_call_as_path_expression() {
    let source = source("fn main(): null { const value = math::random(); return }");

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

    let initializer = const_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CallExpression);
    assert_eq!(source.slice(expression_node.span()), Some("math::random()"));

    let target = expression_node.children()[0];
    let target_node = syntax.node(target).unwrap();

    assert_eq!(target_node.kind(), SyntaxNodeKind::PathExpression);
    assert_eq!(source.slice(target_node.span()), Some("math::random"));

    let left = target_node.children()[0];
    let segment = target_node.children()[1];

    assert_eq!(
        syntax.node(left).unwrap().kind(),
        SyntaxNodeKind::NameExpression
    );
    assert_eq!(
        source.slice(syntax.node(left).unwrap().span()),
        Some("math")
    );

    assert_eq!(
        syntax.node(segment).unwrap().kind(),
        SyntaxNodeKind::Identifier
    );
    assert_eq!(
        source.slice(syntax.node(segment).unwrap().span()),
        Some("random")
    );
}

#[test]
fn parse_anchor_like_call_as_path_expression() {
    let source = source("fn main(): null { const value = user::nameUpper(); return }");

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

    let initializer = const_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CallExpression);
    assert_eq!(
        source.slice(expression_node.span()),
        Some("user::nameUpper()")
    );

    let target = expression_node.children()[0];
    let target_node = syntax.node(target).unwrap();

    assert_eq!(target_node.kind(), SyntaxNodeKind::PathExpression);
    assert_eq!(source.slice(target_node.span()), Some("user::nameUpper"));

    let left = target_node.children()[0];
    let segment = target_node.children()[1];

    assert_eq!(
        syntax.node(left).unwrap().kind(),
        SyntaxNodeKind::NameExpression
    );
    assert_eq!(
        source.slice(syntax.node(left).unwrap().span()),
        Some("user")
    );

    assert_eq!(
        syntax.node(segment).unwrap().kind(),
        SyntaxNodeKind::Identifier
    );
    assert_eq!(
        source.slice(syntax.node(segment).unwrap().span()),
        Some("nameUpper")
    );
}

#[test]
fn parse_chained_path_expression() {
    let source =
        source("fn main(): null { const value = collections::list::first(items); return }");

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

    let initializer = const_node.children()[1];
    let initializer_node = syntax.node(initializer).unwrap();

    let expression = initializer_node.children()[0];
    let expression_node = syntax.node(expression).unwrap();

    assert_eq!(expression_node.kind(), SyntaxNodeKind::CallExpression);

    let target = expression_node.children()[0];
    let target_node = syntax.node(target).unwrap();

    assert_eq!(target_node.kind(), SyntaxNodeKind::PathExpression);
    assert_eq!(
        source.slice(target_node.span()),
        Some("collections::list::first")
    );

    let left = target_node.children()[0];
    let left_node = syntax.node(left).unwrap();

    assert_eq!(left_node.kind(), SyntaxNodeKind::PathExpression);
    assert_eq!(source.slice(left_node.span()), Some("collections::list"));
}
