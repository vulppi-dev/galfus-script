use super::*;

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
