use super::*;
use crate::workspace::{compile_workspace_modules, execute_workspace, load_workspace_for_check};

#[test]
fn workspace_adapter_loads_sources_and_checks_them() -> Result<()> {
    let root = temp_workspace()?;
    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
        [module]
        name = "adapter"
        target = "app"
        entry = "src/main.gfs"
        "#,
    )?;
    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
        import { add } from "./math"

        export fn main(args: [[u8]]): i32 {
            return add(20, 22)
        }
        "#,
    )?;
    write_file(
        root.as_path(),
        "src/math.gfs",
        r#"
        export fn add(left: i32, right: i32): i32 {
            return left + right
        }
        "#,
    )?;

    let mut workspace = load_workspace_for_check(root.as_path())?;
    assert!(workspace.check().is_valid);

    let graph = compile_workspace_modules(root.as_path())?;
    assert_eq!(graph.len(), 2);
    assert_eq!(graph.edges().len(), 1);
    let main = graph
        .modules()
        .find(|module| module.path().as_str() == "src/main.gfs")
        .expect("main module image");
    assert_eq!(main.image().imports[0].module_name, "src/math.gfs");
    assert_eq!(main.image().imports[0].symbol_name, "add");

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn workspace_adapter_executes_single_module_entry() -> Result<()> {
    let root = temp_workspace()?;
    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
        [module]
        name = "adapter-run"
        target = "app"
        entry = "main.gfs"
        "#,
    )?;
    write_file(
        root.as_path(),
        "main.gfs",
        r#"
        export fn main(args: [[u8]]): i32 {
            return 42
        }
        "#,
    )?;

    assert_eq!(execute_workspace(root.as_path(), &[])?, 42);

    fs::remove_dir_all(root)?;
    Ok(())
}
