use galfus_runtime::Runtime;
use galfus_target::WebTarget;
use std::fs;
use std::path::PathBuf;

use crate::workspace::{check_workspace, compile_workspace_to_image};

#[test]
fn path_call_prefers_imported_function_over_same_named_local_function() -> anyhow::Result<()> {
    let root = temp_workspace("galfus_import_call_resolution");
    let src = root.join("src");

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&src)?;

    fs::write(
        root.join("galfus.toml"),
        r#"
[module]
name = "import-call-resolution"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    fs::write(
        src.join("main.gfs"),
        r#"
import a from "./a"

export fn main(args: [[u8]]): i32 {
  return a::value()
}
"#,
    )?;

    fs::write(
        src.join("a.gfs"),
        r#"
import b from "./b"

export fn value(): i32 {
  return b::value()
}
"#,
    )?;

    fs::write(
        src.join("b.gfs"),
        r#"
export fn value(): i32 {
  return 42
}
"#,
    )?;

    let check_result = check_workspace(&root)?;
    assert!(
        !check_result.has_errors(),
        "workspace check failed: {:?}",
        check_result.check_result().diagnostics()
    );

    let image = compile_workspace_to_image(&check_result)?;
    let module_name = image.name.clone();

    let target = WebTarget::new();
    let mut runtime = Runtime::new(Box::new(target));
    runtime.loader().load(image);

    let args: Vec<Vec<u8>> = Vec::new();
    let result = runtime
        .run_entry(module_name.as_str(), "main", &args)
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    assert_eq!(format!("{result:?}"), "42");

    let _ = fs::remove_dir_all(&root);

    Ok(())
}

fn temp_workspace(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
}
