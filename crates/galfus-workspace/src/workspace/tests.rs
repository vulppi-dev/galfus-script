use super::*;

#[test]
fn check_includes_configured_entry_and_exports_as_semantic_roots() {
    let mut workspace = Workspace::new();
    workspace
        .load_config(
            br#"
            [module]
            name = "semantic-roots"
            target = "app"
            entry = "main.gfs"

            [exports]
            library = "library.gfs"
            "#,
        )
        .expect("valid configuration");
    workspace
        .load_module(
            "main.gfs",
            b"export fn main(args: [[u8]]): i32 { return 0 }",
        )
        .expect("valid entry module");
    workspace
        .load_module("library.gfs", b"export fn value(): i32 { return 1 }")
        .expect("valid export module");

    assert!(workspace.check().is_valid);

    let roots = workspace.frontend.semantic_graph().roots();
    assert!(roots.iter().any(|root| {
        root.kind() == &SemanticRootKind::Entry && root.path().as_str() == "main.gfs"
    }));
    assert!(roots.iter().any(|root| {
        root.kind()
            == &SemanticRootKind::Export {
                address: "library".to_string(),
            }
            && root.path().as_str() == "library.gfs"
    }));
}

#[test]
fn compile_emits_one_image_per_module_with_import_slots() {
    let mut workspace = Workspace::new();
    workspace
        .load_config(
            br#"
            [module]
            name = "module-images"
            target = "app"
            entry = "main.gfs"
            "#,
        )
        .expect("valid configuration");
    workspace
        .load_module(
            "main.gfs",
            br#"
            import { add } from "./math"

            export fn main(args: [[u8]]): i32 {
                return add(20, 22)
            }
            "#,
        )
        .expect("valid main module");
    workspace
        .load_module(
            "math.gfs",
            br#"
            export fn add(left: i32, right: i32): i32 {
                return left + right
            }
            "#,
        )
        .expect("valid dependency module");

    assert!(workspace.check().is_valid);
    let report = workspace.compile().expect("workspace compiles");

    assert_eq!(report.graph.len(), 2);
    assert_eq!(report.graph.edges().len(), 1);

    let main = report
        .graph
        .modules()
        .find(|image| image.path().as_str() == "main.gfs")
        .expect("main image");
    assert_eq!(main.image().imports.len(), 1);
    assert_eq!(main.image().imports[0].module_name, "math.gfs");
    assert_eq!(main.image().imports[0].symbol_name, "add");
    assert!(
        main.image()
            .functions
            .iter()
            .all(|function| function.name != "__init_workspace")
    );
}

#[test]
fn compile_updates_changed_images_and_removes_deleted_images() {
    let mut workspace = Workspace::new();
    workspace
        .load_config(
            br#"
            [module]
            name = "incremental-compile"
            target = "app"
            entry = "main.gfs"
            "#,
        )
        .expect("valid configuration");
    workspace
        .load_module(
            "main.gfs",
            br#"
            export fn main(args: [[u8]]): i32 {
                return 0
            }
            "#,
        )
        .expect("valid main module");
    workspace
        .load_module(
            "helper.gfs",
            br#"
            export fn value(): i32 {
                return 1
            }
            "#,
        )
        .expect("valid helper module");

    assert!(workspace.check().is_valid);
    let first_graph = workspace.compile().expect("initial compilation").graph;
    let main = first_graph
        .modules()
        .find(|image| image.path().as_str() == "main.gfs")
        .expect("main image");
    let helper = first_graph
        .modules()
        .find(|image| image.path().as_str() == "helper.gfs")
        .expect("helper image");
    let main_id = main.id();
    let helper_id = helper.id();
    let main_revision = main.semantic_revision();
    let helper_revision = helper.semantic_revision();

    workspace
        .load_module(
            "helper.gfs",
            br#"
            export fn value(): i32 {
                return 2
            }
            "#,
        )
        .expect("updated helper module");
    assert!(workspace.check().is_valid);
    let updated_graph = workspace.compile().expect("incremental compilation").graph;

    assert_eq!(
        updated_graph
            .get(main_id)
            .expect("cached main image")
            .semantic_revision(),
        main_revision
    );
    assert!(
        updated_graph
            .get(helper_id)
            .expect("updated helper image")
            .semantic_revision()
            > helper_revision
    );

    assert!(matches!(
        workspace.remove_module("helper.gfs"),
        Ok(RemoveResult::Success)
    ));
    assert!(workspace.check().is_valid);
    let deleted_graph = workspace
        .compile()
        .expect("compilation after deletion")
        .graph;

    assert_eq!(deleted_graph.len(), 1);
    assert!(deleted_graph.get(helper_id).is_none());
    assert_eq!(
        deleted_graph
            .get(main_id)
            .expect("cached main image")
            .semantic_revision(),
        main_revision
    );
}

#[test]
fn run_requires_compile_and_executes_the_configured_entry() {
    let mut workspace = Workspace::new();
    assert!(matches!(
        workspace.run(&[]),
        Err(RunBlocked::CompileRequired)
    ));

    workspace
        .load_config(
            br#"
            [module]
            name = "run-entry"
            target = "app"
            entry = "main.gfs"
            "#,
        )
        .expect("valid configuration");
    workspace
        .load_module(
            "main.gfs",
            br#"
            export fn main(args: [[u8]]): i32 {
                return 42
            }
            "#,
        )
        .expect("valid entry module");

    assert!(matches!(
        workspace.compile(),
        Err(CompileBlocked::Dirty { .. })
    ));
    assert!(workspace.check().is_valid);
    workspace.compile().expect("workspace compiles");
    assert_eq!(workspace.run(&[]).expect("entry executes").exit_code, 42);
}
