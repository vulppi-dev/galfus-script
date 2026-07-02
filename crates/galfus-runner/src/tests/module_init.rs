use crate::{check_workspace, compile_workspace_to_image};
use galfus_runtime::Runtime;
use galfus_target::WebTarget;
use std::fs;
use std::path::PathBuf;

#[test]
fn workspace_init_runs_dependencies_before_importers() -> anyhow::Result<()> {
    let root = temp_workspace("galfus_init_order");
    let src = root.join("src");

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&src)?;

    fs::write(
        root.join("galfus.toml"),
        r#"
[module]
name = "init-order"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    fs::write(
        src.join("main.gfs"),
        r#"
import io from "std/io"
import a from "./a"

var initMain = io::print("M")

export fn main(args: [[uint8]]): int32 {
  a::touch()
  return 0
}
"#,
    )?;

    fs::write(
        src.join("a.gfs"),
        r#"
import io from "std/io"
import b from "./b"

var initA = io::print("A")

export fn touch(): null {
  b::touch()
  return
}
"#,
    )?;

    fs::write(
        src.join("b.gfs"),
        r#"
import io from "std/io"

var initB = io::print("B")

export fn touch(): null {
  return
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
    let output_target = target.clone();

    let mut runtime = Runtime::new(Box::new(target));
    runtime.loader().load(image);

    let args: Vec<Vec<u8>> = Vec::new();
    runtime
        .run_entry(module_name.as_str(), "main", &args)
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    let output = String::from_utf8(output_target.take_output())?;
    assert_eq!(output, "BAM");

    let _ = fs::remove_dir_all(&root);

    Ok(())
}

fn temp_workspace(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
}
