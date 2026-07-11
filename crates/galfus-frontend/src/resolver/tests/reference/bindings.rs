use super::*;

#[test]
fn resolve_binds_parameter_name_expression() {
    let source = source(
        r#"
        fn main(value: i32): null {
            var result = value
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(
        !resolve_result.has_errors(),
        "{:?}",
        resolve_result.diagnostics()
    );

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "value").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "value");
    assert_eq!(symbol.kind(), SymbolKind::Parameter);
}

#[test]
fn resolve_declares_arrow_function_parameter_in_arrow_scope() {
    let source = source(
        r#"
        fn main(): null {
            const double = (value: i32): i32 => value * 2
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(
        !resolve_result.has_errors(),
        "{:?}",
        resolve_result.diagnostics()
    );

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();
    let arrow = find_node_by_kind(syntax, root, SyntaxNodeKind::ArrowFunctionExpression).unwrap();
    let arrow_scope = resolution
        .scope(resolution.node_scope(arrow).unwrap())
        .unwrap();
    let parameter_symbol = resolution
        .symbol(arrow_scope.symbol("value").unwrap())
        .unwrap();

    assert_eq!(arrow_scope.kind(), ScopeKind::ArrowFunction);
    assert_eq!(parameter_symbol.kind(), SymbolKind::Parameter);

    let expression = find_name_expression_by_text(syntax, &source, root, "value").unwrap();
    let reference_symbol = resolution
        .symbol(resolution.reference_symbol(expression).unwrap())
        .unwrap();

    assert_eq!(reference_symbol.name(), "value");
    assert_eq!(reference_symbol.kind(), SymbolKind::Parameter);
    assert_eq!(reference_symbol.scope(), arrow_scope.id());
}

#[test]
fn resolve_arrow_function_parameter_reaches_block_body() {
    let source = source(
        r#"
        fn print(value: i32): null {
            return
        }

        fn main(): null {
            const logValue = (value: i32): null => {
                print(value)
                return
            }
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(
        !resolve_result.has_errors(),
        "{:?}",
        resolve_result.diagnostics()
    );

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();
    let arrow = find_node_by_kind(syntax, root, SyntaxNodeKind::ArrowFunctionExpression).unwrap();
    let arrow_scope_id = resolution.node_scope(arrow).unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "value").unwrap();
    let reference_symbol = resolution
        .symbol(resolution.reference_symbol(expression).unwrap())
        .unwrap();

    assert_eq!(reference_symbol.name(), "value");
    assert_eq!(reference_symbol.kind(), SymbolKind::Parameter);
    assert_eq!(reference_symbol.scope(), arrow_scope_id);
}

#[test]
fn resolve_arrow_function_body_can_capture_parent_scope_name() {
    let source = source(
        r#"
        fn main(offset: i32): null {
            const addOffset = (value: i32): i32 => value + offset
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(
        !resolve_result.has_errors(),
        "{:?}",
        resolve_result.diagnostics()
    );

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();
    let expression = find_name_expression_by_text(syntax, &source, root, "offset").unwrap();
    let reference_symbol = resolution
        .symbol(resolution.reference_symbol(expression).unwrap())
        .unwrap();

    assert_eq!(reference_symbol.name(), "offset");
    assert_eq!(reference_symbol.kind(), SymbolKind::Parameter);
}

#[test]
fn resolve_reports_duplicate_arrow_function_parameter() {
    let source = source(
        r#"
        fn main(): null {
            const duplicate = (value: i32, value: i32): i32 => value
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();

    assert!(
        graph
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.message().contains("duplicate symbol `value`") })
    );
}

#[test]
fn resolve_declares_for_binding_in_for_scope() {
    let source = source(
        r#"
        fn print(value: i32): null {
            return
        }

        fn main(items: [i32]): null {
            for item in items {
                print(item)
            }

            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(
        !resolve_result.has_errors(),
        "{:?}",
        resolve_result.diagnostics()
    );

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();
    let for_statement = find_node_by_kind(syntax, root, SyntaxNodeKind::ForStatement).unwrap();
    let for_scope = resolution
        .scope(resolution.node_scope(for_statement).unwrap())
        .unwrap();
    let binding_symbol = resolution
        .symbol(for_scope.symbol("item").unwrap())
        .unwrap();

    assert_eq!(for_scope.kind(), ScopeKind::For);
    assert_eq!(binding_symbol.kind(), SymbolKind::ForBinding);

    let expression = find_name_expression_by_text(syntax, &source, root, "item").unwrap();
    let reference_symbol = resolution
        .symbol(resolution.reference_symbol(expression).unwrap())
        .unwrap();

    assert_eq!(reference_symbol.name(), "item");
    assert_eq!(reference_symbol.kind(), SymbolKind::ForBinding);
}

#[test]
fn resolve_for_iterable_uses_parent_scope_before_for_binding_scope() {
    let source = source(
        r#"
        fn print(value: i32): null {
            return
        }

        fn main(items: [i32]): null {
            for items in items {
                print(items)
            }

            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(
        !resolve_result.has_errors(),
        "{:?}",
        resolve_result.diagnostics()
    );

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();
    let iterable = find_name_expression_by_text(syntax, &source, root, "items").unwrap();
    let reference_symbol = resolution
        .symbol(resolution.reference_symbol(iterable).unwrap())
        .unwrap();

    assert_eq!(reference_symbol.name(), "items");
    assert_eq!(reference_symbol.kind(), SymbolKind::Parameter);
}

#[test]
fn resolve_reports_unknown_for_iterable_name() {
    let source = source(
        r#"
        fn main(): null {
            for item in missing {
            }

            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();

    assert!(
        graph
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.message().contains("unresolved name `missing`") })
    );
}
