use super::*;
use crate::SyntaxLayer;
use galfus_core::NodeId;

fn find_name_expression_by_text(
    syntax: &SyntaxLayer,
    source: &SourceFile,
    node: NodeId,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = syntax.node(node)?;

    if syntax_node.kind() == SyntaxNodeKind::NameExpression {
        if let Some(identifier) = syntax.first_child_of_kind(node, SyntaxNodeKind::Identifier) {
            if source.slice(syntax.node(identifier)?.span()) == Some(text) {
                return Some(node);
            }
        }
    }

    for child in syntax_node.children() {
        if let Some(found) = find_name_expression_by_text(syntax, source, *child, text) {
            return Some(found);
        }
    }

    None
}

fn find_path_expression_by_text(
    syntax: &SyntaxLayer,
    source: &SourceFile,
    node: NodeId,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = syntax.node(node)?;

    if syntax_node.kind() == SyntaxNodeKind::PathExpression
        && source.slice(syntax_node.span()) == Some(text)
    {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_path_expression_by_text(syntax, source, *child, text) {
            return Some(found);
        }
    }

    None
}

fn find_node_by_kind_and_text(
    syntax: &SyntaxLayer,
    source: &SourceFile,
    node: NodeId,
    kind: SyntaxNodeKind,
    text: &str,
) -> Option<NodeId> {
    let syntax_node = syntax.node(node)?;

    if syntax_node.kind() == kind && source.slice(syntax_node.span()) == Some(text) {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_node_by_kind_and_text(syntax, source, *child, kind, text) {
            return Some(found);
        }
    }

    None
}

#[test]
fn resolve_binds_parameter_name_expression() {
    let source = source(
        r#"
        fn main(value: int32): null {
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
fn resolve_declares_match_binding_pattern_in_arm_scope() {
    let source = source(
        r#"
        fn main(value: int32): int32 {
            return match value {
                other => other,
            }
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
    let arm = find_node_by_kind_and_text(
        syntax,
        &source,
        root,
        SyntaxNodeKind::MatchArm,
        "other => other",
    )
    .unwrap();

    let arm_scope = resolution
        .scope(resolution.node_scope(arm).unwrap())
        .unwrap();
    let binding_symbol = resolution
        .symbol(arm_scope.symbol("other").unwrap())
        .unwrap();

    assert_eq!(binding_symbol.kind(), SymbolKind::PatternBinding);

    let expression = find_name_expression_by_text(syntax, &source, root, "other").unwrap();
    let reference_symbol = resolution
        .symbol(resolution.reference_symbol(expression).unwrap())
        .unwrap();

    assert_eq!(reference_symbol.name(), "other");
    assert_eq!(reference_symbol.kind(), SymbolKind::PatternBinding);
}

#[test]
fn resolve_declares_choice_payload_binding_in_arm_scope() {
    let source = source(
        r#"
        choice Result<V, F> {
            Ok(V),
            Err(F),
        }

        fn main(result: Result<int32, [int8]>): int32 {
            return match result {
                Result::Ok(value) => value,
                Result::Err(error) => 0,
            }
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
    let arm = find_node_by_kind_and_text(
        syntax,
        &source,
        root,
        SyntaxNodeKind::MatchArm,
        "Result::Ok(value) => value",
    )
    .unwrap();

    let arm_scope = resolution
        .scope(resolution.node_scope(arm).unwrap())
        .unwrap();
    let binding_symbol = resolution
        .symbol(arm_scope.symbol("value").unwrap())
        .unwrap();

    assert_eq!(binding_symbol.kind(), SymbolKind::PatternBinding);

    let expression = find_name_expression_by_text(syntax, &source, root, "value").unwrap();
    let reference_symbol = resolution
        .symbol(resolution.reference_symbol(expression).unwrap())
        .unwrap();

    assert_eq!(reference_symbol.name(), "value");
    assert_eq!(reference_symbol.kind(), SymbolKind::PatternBinding);
}

#[test]
fn resolve_binds_variant_pattern_member() {
    let source = source(
        r#"
        enum Color {
            Red,
            Blue,
        }

        fn main(color: Color): int32 {
            return match color {
                Color::Red => 1,
                Color::Blue => 2,
            }
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();
    let pattern = find_node_by_kind_and_text(
        syntax,
        &source,
        root,
        SyntaxNodeKind::VariantPattern,
        "Color::Red",
    )
    .unwrap();

    let root_symbol = resolution
        .symbol(resolution.reference_symbol(pattern).unwrap())
        .unwrap();
    let member_symbol = resolution
        .symbol(resolution.path_reference_symbol(pattern).unwrap())
        .unwrap();

    assert_eq!(root_symbol.name(), "Color");
    assert_eq!(root_symbol.kind(), SymbolKind::Enum);

    assert_eq!(member_symbol.name(), "Red");
    assert_eq!(member_symbol.kind(), SymbolKind::EnumVariant);
}

#[test]
fn resolve_reports_unknown_variant_pattern_member() {
    let source = source(
        r#"
        enum Color {
            Red,
        }

        fn main(color: Color): int32 {
            return match color {
                Color::Blue => 2,
            }
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();

    assert!(graph.diagnostics().iter().any(|diagnostic| {
        diagnostic
            .message()
            .contains("unresolved path member `Blue`")
    }));
}

#[test]
fn resolve_binds_block_local_name_expression() {
    let source = source(
        r#"
        fn main(): null {
            var first = 1
            var second = first
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "first").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "first");
    assert_eq!(symbol.kind(), SymbolKind::Var);
}

#[test]
fn resolve_binds_name_from_parent_block_scope() {
    let source = source(
        r#"
        fn main(): null {
            var outer = 1

            if true {
                var inner = outer
            }

            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "outer").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "outer");
    assert_eq!(symbol.kind(), SymbolKind::Var);
}

#[test]
fn resolve_prefers_nearest_scope_symbol() {
    let source = source(
        r#"
        fn main(): null {
            var value = 1

            if true {
                var value = 2
                var result = value
            }

            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "value").unwrap();

    let referenced_symbol_id = resolution.reference_symbol(expression).unwrap();
    let referenced_symbol = resolution.symbol(referenced_symbol_id).unwrap();

    let function = syntax.first_child(root).unwrap();

    let outer_block = syntax
        .first_child_of_kind(function, SyntaxNodeKind::Block)
        .unwrap();

    let if_statement = syntax
        .first_child_of_kind(outer_block, SyntaxNodeKind::IfStatement)
        .unwrap();

    let inner_block = syntax
        .first_child_of_kind(if_statement, SyntaxNodeKind::Block)
        .unwrap();

    let inner_block_scope = resolution.node_scope(inner_block).unwrap();

    assert_eq!(referenced_symbol.name(), "value");
    assert_eq!(referenced_symbol.kind(), SymbolKind::Var);
    assert_eq!(referenced_symbol.scope(), inner_block_scope);
}

#[test]
fn resolve_binds_import_namespace_name_expression() {
    let source = source(
        r#"
        import user from "./user"

        fn main(): null {
            var current = user
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "user").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "user");
    assert_eq!(symbol.kind(), SymbolKind::ImportNamespace);
}

#[test]
fn resolve_binds_import_namespace_path_expression_root() {
    let source = source(
        r#"
        import user from "./user"

        fn main(): null {
            var created = user::create()
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_path_expression_by_text(syntax, &source, root, "user::create").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "user");
    assert_eq!(symbol.kind(), SymbolKind::ImportNamespace);
}

#[test]
fn resolve_binds_local_path_expression_root() {
    let source = source(
        r#"
        fn main(): null {
            var local = 0
            var current = local::member
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_path_expression_by_text(syntax, &source, root, "local::member").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "local");
    assert_eq!(symbol.kind(), SymbolKind::Var);
}

#[test]
fn resolve_binds_enum_variant_path_expression_member() {
    let source = source(
        r#"
        enum Status {
            Off,
            On,
        }

        fn main(): null {
            var current = Status::On
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_path_expression_by_text(syntax, &source, root, "Status::On").unwrap();

    let root_symbol = resolution
        .symbol(resolution.reference_symbol(expression).unwrap())
        .unwrap();
    let member_symbol = resolution
        .symbol(resolution.path_reference_symbol(expression).unwrap())
        .unwrap();

    assert_eq!(root_symbol.name(), "Status");
    assert_eq!(root_symbol.kind(), SymbolKind::Enum);

    assert_eq!(member_symbol.name(), "On");
    assert_eq!(member_symbol.kind(), SymbolKind::EnumVariant);
}

#[test]
fn resolve_binds_choice_variant_path_expression_member() {
    let source = source(
        r#"
        choice Result<V, F> {
            Ok(V),
            Err(F),
        }

        fn main(value: int32): null {
            var current = Result::Ok
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_path_expression_by_text(syntax, &source, root, "Result::Ok").unwrap();

    let member_symbol = resolution
        .symbol(resolution.path_reference_symbol(expression).unwrap())
        .unwrap();

    assert_eq!(member_symbol.name(), "Ok");
    assert_eq!(member_symbol.kind(), SymbolKind::ChoiceVariant);
}

#[test]
fn resolve_binds_constraint_function_path_expression_member() {
    let source = source(
        r#"
        constraint Stringable<T> {
            fn toString(self: T): [int8]
        }

        fn main(): null {
            var method = Stringable::toString
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression =
        find_path_expression_by_text(syntax, &source, root, "Stringable::toString").unwrap();

    let root_symbol = resolution
        .symbol(resolution.reference_symbol(expression).unwrap())
        .unwrap();
    let member_symbol = resolution
        .symbol(resolution.path_reference_symbol(expression).unwrap())
        .unwrap();

    assert_eq!(root_symbol.name(), "Stringable");
    assert_eq!(root_symbol.kind(), SymbolKind::Constraint);

    assert_eq!(member_symbol.name(), "toString");
    assert_eq!(member_symbol.kind(), SymbolKind::ConstraintFunction);
}

#[test]
fn resolve_reports_unknown_path_expression_member_on_local_type() {
    let source = source(
        r#"
        enum Status {
            Off,
            On,
        }

        fn main(): null {
            var current = Status::Missing
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression =
        find_path_expression_by_text(syntax, &source, root, "Status::Missing").unwrap();

    assert!(resolution.reference_symbol(expression).is_some());
    assert!(resolution.path_reference_symbol(expression).is_none());

    assert!(graph.diagnostics().iter().any(|diagnostic| {
        diagnostic
            .message()
            .contains("unresolved path member `Missing`")
    }));
}

#[test]
fn resolve_reports_unknown_path_expression_root() {
    let source = source(
        r#"
        fn main(): null {
            var created = missing::create()
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression =
        find_path_expression_by_text(syntax, &source, root, "missing::create").unwrap();

    assert!(resolution.reference_symbol(expression).is_none());

    assert!(
        graph
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("unresolved name `missing`"))
    );
}

#[test]
fn resolve_binds_top_level_initializer_name_expression() {
    let source = source(
        r#"
        const first = 1
        const second = first
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "first").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "first");
    assert_eq!(symbol.kind(), SymbolKind::Const);
}

#[test]
fn resolve_binds_parameter_default_name_expression() {
    let source = source(
        r#"
        const fallback = 1

        fn main(value: int32 = fallback): null {
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "fallback").unwrap();

    let symbol = resolution.reference_symbol(expression).unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.name(), "fallback");
    assert_eq!(symbol.kind(), SymbolKind::Const);
}

#[test]
fn resolve_reports_unknown_top_level_initializer_name_expression() {
    let source = source(
        r#"
        const value = missing
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "missing").unwrap();

    assert!(resolution.reference_symbol(expression).is_none());

    assert!(
        graph
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("unresolved name `missing`"))
    );
}

#[test]
fn resolve_reports_unknown_name_expression() {
    let source = source(
        r#"
        fn main(): null {
            var value = missing
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "missing").unwrap();

    assert!(resolution.reference_symbol(expression).is_none());

    assert!(
        graph
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("unresolved name `missing`"))
    );
}

#[test]
fn resolve_does_not_bind_builtin_type_as_value_name() {
    let source = source(
        r#"
        fn main(): null {
            var value = int8
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let syntax = graph.syntax();
    let resolution = graph.resolution().unwrap();

    let root = syntax.root().unwrap();

    let expression = find_name_expression_by_text(syntax, &source, root, "int8").unwrap();

    assert!(resolution.reference_symbol(expression).is_none());
}
