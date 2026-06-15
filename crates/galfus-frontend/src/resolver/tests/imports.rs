use super::*;

#[test]
fn resolve_declares_namespace_import_binding() {
    let source = source(
        r#"
        import user from "./user"

        fn main(): null {
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let resolution = graph.resolution().unwrap();
    let module_scope = resolution.scope(resolution.module_scope()).unwrap();

    let symbol = module_scope.symbol("user").unwrap();
    let symbol = resolution.symbol(symbol).unwrap();

    assert_eq!(symbol.kind(), SymbolKind::ImportNamespace);
    assert_eq!(symbol.name(), "user");
}

#[test]
fn resolve_declares_named_import_bindings() {
    let source = source(
        r#"
        import { User, create } from "./user"

        fn main(): null {
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let resolution = graph.resolution().unwrap();
    let module_scope = resolution.scope(resolution.module_scope()).unwrap();

    let user = resolution
        .symbol(module_scope.symbol("User").unwrap())
        .unwrap();

    let create = resolution
        .symbol(module_scope.symbol("create").unwrap())
        .unwrap();

    assert_eq!(user.kind(), SymbolKind::ImportBinding);
    assert_eq!(create.kind(), SymbolKind::ImportBinding);
}

#[test]
fn resolve_declares_named_import_alias_binding() {
    let source = source(
        r#"
        import { User as LocalUser } from "./user"

        fn main(): null {
            return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();
    let resolution = graph.resolution().unwrap();
    let module_scope = resolution.scope(resolution.module_scope()).unwrap();

    assert!(module_scope.symbol("User").is_none());

    let local_user = resolution
        .symbol(module_scope.symbol("LocalUser").unwrap())
        .unwrap();

    assert_eq!(local_user.kind(), SymbolKind::ImportBinding);
    assert_eq!(local_user.name(), "LocalUser");
}

#[test]
fn resolve_reports_duplicate_import_and_local_symbol() {
    let source = source(
        r#"
        fn user(): null {
            return
        }

        import user from "./user"
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());

    let graph = resolve_result.graph();
    let resolution = graph.resolution().unwrap();
    let module_scope = resolution.scope(resolution.module_scope()).unwrap();

    let symbol = resolution
        .symbol(module_scope.symbol("user").unwrap())
        .unwrap();

    assert_eq!(symbol.kind(), SymbolKind::Function);
}
