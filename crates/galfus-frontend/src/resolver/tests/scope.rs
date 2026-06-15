use super::*;

#[test]
fn resolve_creates_module_scope() {
    let source = source("const version = \"0.1.0\"");

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.graph();

    assert_eq!(graph.phase(), GraphPhase::Resolved);

    let resolution = graph.resolution().unwrap();
    let module_scope = resolution.module_scope();

    assert_eq!(resolution.scopes().len(), 1);
    assert_eq!(
        resolution.scope(module_scope).unwrap().kind(),
        ScopeKind::Module
    );
}
