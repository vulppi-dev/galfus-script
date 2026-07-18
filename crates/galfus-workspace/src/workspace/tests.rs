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
fn compile_rebuilds_only_changed_modules_and_transitive_dependents() {
    let mut workspace = Workspace::new();
    workspace
        .load_config(
            br#"
            [module]
            name = "dependent-compile"
            target = "app"
            entry = "main.gfs"
            "#,
        )
        .expect("valid configuration");
    workspace
        .load_module(
            "main.gfs",
            br#"
            import { value } from "./dependency"

            export fn main(args: [[u8]]): i32 {
                return value()
            }
            "#,
        )
        .expect("valid entry module");
    workspace
        .load_module(
            "dependency.gfs",
            br#"
            export fn value(): i32 {
                return 1
            }
            "#,
        )
        .expect("valid dependency module");
    workspace
        .load_module(
            "isolated.gfs",
            br#"
            export fn isolated(): i32 {
                return 0
            }
            "#,
        )
        .expect("valid isolated module");

    assert!(workspace.check().is_valid);
    let first = workspace.compile().expect("initial compilation").graph;
    let main = first
        .modules()
        .find(|image| image.path().as_str() == "main.gfs")
        .expect("main image");
    let dependency = first
        .modules()
        .find(|image| image.path().as_str() == "dependency.gfs")
        .expect("dependency image");
    let isolated = first
        .modules()
        .find(|image| image.path().as_str() == "isolated.gfs")
        .expect("isolated image");
    let main_revision = main.semantic_revision();
    let dependency_revision = dependency.semantic_revision();
    let isolated_revision = isolated.semantic_revision();
    let main_id = main.id();
    let dependency_id = dependency.id();
    let isolated_id = isolated.id();

    workspace
        .load_module(
            "dependency.gfs",
            br#"
            export fn value(): i32 {
                return 2
            }
            "#,
        )
        .expect("updated dependency module");
    assert!(workspace.check().is_valid);
    let updated = workspace.compile().expect("incremental compilation").graph;

    assert!(
        updated
            .get(main_id)
            .expect("recompiled main")
            .semantic_revision()
            > main_revision
    );
    assert!(
        updated
            .get(dependency_id)
            .expect("recompiled dependency")
            .semantic_revision()
            > dependency_revision
    );
    assert_eq!(
        updated
            .get(isolated_id)
            .expect("cached isolated module")
            .semantic_revision(),
        isolated_revision
    );
}

#[test]
fn run_requires_compile_and_executes_the_configured_entry() {
    let mut workspace = Workspace::new();
    assert!(matches!(
        workspace.run(&[], None),
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
    assert_eq!(
        workspace.run(&[], None).expect("entry executes").exit_code,
        42
    );
}

#[test]
fn run_reports_missing_io_provider_only_when_io_is_executed() {
    let mut workspace = Workspace::new();
    workspace
        .load_config(
            br#"
            [module]
            name = "missing-io-provider"
            target = "app"
            entry = "main.gfs"
            "#,
        )
        .expect("valid configuration");
    workspace
        .load_module(
            "main.gfs",
            br#"
            import { println } from "std/io"

            export fn main(args: [[u8]]): i32 {
                println("output")
                return 0
            }
            "#,
        )
        .expect("valid entry module");

    assert!(workspace.check().is_valid);
    workspace.compile().expect("workspace compiles");

    let error = match workspace.run(&[], None) {
        Err(error) => error,
        Ok(_) => panic!("I/O requires a provider at runtime"),
    };
    assert!(matches!(
        error,
        RunBlocked::RuntimeError(message) if message.contains("I/O provider is unavailable for write")
    ));
}

#[test]
fn run_specializes_nested_generic_types_across_modules() {
    let mut workspace = Workspace::new();
    workspace
        .load_config(
            br#"
            [module]
            name = "cross-module-nested-generics"
            target = "app"
            entry = "main.gfs"
            "#,
        )
        .expect("valid configuration");
    workspace
        .load_module(
            "main.gfs",
            br#"
            import { identity } from "./generic"

            export fn main(args: [[u8]]): i32 {
                var values: [i32] = [32]
                return identity(values).length + 41
            }
            "#,
        )
        .expect("valid entry module");
    workspace
        .load_module(
            "generic.gfs",
            br#"
            export fn identity<T>(values: [T]): [T] {
                return values
            }
            "#,
        )
        .expect("valid generic module");

    let check = workspace.check();
    assert!(check.is_valid, "check diagnostics: {:?}", check.diagnostics);
    workspace.compile().expect("workspace compiles");
    assert_eq!(
        workspace.run(&[], None).expect("entry executes").exit_code,
        42
    );
}

#[test]
fn run_specializes_explicit_imported_generic_typeof_parameter() {
    let mut workspace = Workspace::new();
    workspace
        .load_config(
            br#"
            [module]
            name = "cross-module-typeof"
            target = "app"
            entry = "main.gfs"
            "#,
        )
        .expect("valid configuration");
    workspace
        .load_module(
            "main.gfs",
            br#"
            import { dispatch } from "./generic"

            export fn main(args: [[u8]]): i32 {
                return dispatch<i32>(0)
            }
            "#,
        )
        .expect("valid entry module");
    workspace
        .load_module(
            "generic.gfs",
            br#"
            export fn dispatch<T: i32 | bool>(value: T): i32 {
                return typeof T {
                    i32 => 42,
                    bool => 0,
                }
            }
            "#,
        )
        .expect("valid generic module");

    let check = workspace.check();
    assert!(check.is_valid, "check diagnostics: {:?}", check.diagnostics);
    workspace.compile().expect("workspace compiles");
    assert_eq!(
        workspace.run(&[], None).expect("entry executes").exit_code,
        42
    );
}

#[test]
fn run_specializes_generic_anchored_range_iterator_methods() {
    let mut workspace = Workspace::new();
    workspace
        .load_config(
            br#"
            [module]
            name = "generic-range-method"
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
                var total = 0
                for value in 2::4%2 {
                    total += value
                }
                return total
            }
            "#,
        )
        .expect("valid entry module");

    let check = workspace.check();
    assert!(check.is_valid, "check diagnostics: {:?}", check.diagnostics);
    workspace.compile().expect("workspace compiles");
    assert_eq!(
        workspace.run(&[], None).expect("entry executes").exit_code,
        20
    );
}

#[test]
fn run_synchronizes_the_runtime_module_graph() {
    let mut workspace = Workspace::new();
    workspace
        .load_config(
            br#"
            [module]
            name = "runtime-sync"
            target = "app"
            entry = "main.gfs"
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
        .load_module("helper.gfs", b"export fn helper(): i32 { return 1 }")
        .expect("valid helper module");

    assert!(workspace.check().is_valid);
    let first = workspace.compile().expect("workspace compiles").graph;
    let main = first
        .modules()
        .find(|image| image.path().as_str() == "main.gfs")
        .expect("main image");
    let helper = first
        .modules()
        .find(|image| image.path().as_str() == "helper.gfs")
        .expect("helper image");
    let main_id = main.id();
    let helper_id = helper.id();

    assert_eq!(
        workspace.run(&[], None).expect("entry executes").exit_code,
        0
    );
    assert_eq!(workspace.runtime.modules().len(), 2);

    assert!(matches!(
        workspace.remove_module("helper.gfs"),
        Ok(RemoveResult::Success)
    ));
    assert!(workspace.check().is_valid);
    workspace.compile().expect("workspace recompiles");
    assert_eq!(
        workspace.run(&[], None).expect("entry executes").exit_code,
        0
    );
    assert!(workspace.runtime.modules().get(main_id).is_some());
    assert!(workspace.runtime.modules().get(helper_id).is_none());
}
