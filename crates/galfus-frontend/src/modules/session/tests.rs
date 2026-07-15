use super::*;

fn path(value: &str) -> ModulePath {
    ModulePath::new(value).expect("valid module path")
}

#[test]
fn check_uses_the_module_ids_provided_by_the_host() {
    let utilities = SourceFile::new(
        SourceId::new(3),
        "src/utilities.gfs".to_string(),
        "export fn value(): i32 { return 1 }".to_string(),
    );
    let main = SourceFile::new(
        SourceId::new(9),
        "src/main.gfs".to_string(),
        "import { value } from './utilities'\nfn main(): i32 { return value() }".to_string(),
    );
    let sources = [
        FrontendSource {
            module_id: ModuleId::new(41),
            path: path("src/main.gfs"),
            source: &main,
        },
        FrontendSource {
            module_id: ModuleId::new(7),
            path: path("src/utilities.gfs"),
            source: &utilities,
        },
    ];
    let roots = FrontendRoots::default();
    let mut session = FrontendSession::new();

    let report = session.check(FrontendUpdate {
        source_revision: Revision::new(1),
        sources: &sources,
        roots: &roots,
    });

    assert!(!report.diagnostics.has_errors());
    assert_eq!(session.modules[0].id(), ModuleId::new(41));
    assert_eq!(session.modules[1].id(), ModuleId::new(7));
    assert_eq!(
        session
            .semantic_graph()
            .module_by_path(&path("src/main.gfs")),
        Some(ModuleId::new(41))
    );
    assert_eq!(
        session
            .semantic_graph()
            .semantic_revision(ModuleId::new(41)),
        Some(session.modules[0].semantic_revision())
    );
    assert!(
        session.semantic_graph().import_edges().iter().any(|edge| {
            edge.from() == ModuleId::new(41) && edge.to() == Some(ModuleId::new(7))
        })
    );
}
