use crate::check_workspace;
use std::fs;
use std::path::PathBuf;

#[test]
fn user_file_cannot_declare_builtin_function_key() -> anyhow::Result<()> {
    let root = temp_workspace("galfus_reserved_builtin_declare_fn");
    let src = root.join("src");

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&src)?;

    write_galfus_toml(&root)?;

    fs::write(
        src.join("main.gfs"),
        r#"
fn __builtin_write(text: [uint8]): null {
  return
}

export fn main(args: [[uint8]]): int32 {
  return 0
}
"#,
    )?;

    let check_result = check_workspace(&root)?;
    assert!(
        check_result.has_errors(),
        "expected user declaration of __builtin_write to be rejected"
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn user_file_cannot_reference_builtin_key_directly() -> anyhow::Result<()> {
    let root = temp_workspace("galfus_reserved_builtin_reference");
    let src = root.join("src");

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&src)?;

    write_galfus_toml(&root)?;

    fs::write(
        src.join("main.gfs"),
        r#"
export fn main(args: [[uint8]]): int32 {
  __builtin_write("x")
  return 0
}
"#,
    )?;

    let check_result = check_workspace(&root)?;
    assert!(
        check_result.has_errors(),
        "expected direct user reference to __builtin_write to be rejected"
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn user_file_cannot_declare_reserved_builtin_variable() -> anyhow::Result<()> {
    let root = temp_workspace("galfus_reserved_builtin_var");
    let src = root.join("src");

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&src)?;

    write_galfus_toml(&root)?;

    fs::write(
        src.join("main.gfs"),
        r#"
var __builtin_value = 1

export fn main(args: [[uint8]]): int32 {
  return 0
}
"#,
    )?;

    let check_result = check_workspace(&root)?;
    assert!(
        check_result.has_errors(),
        "expected user declaration of __builtin_value to be rejected"
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn std_io_builtin_surface_still_works_for_user_code() -> anyhow::Result<()> {
    let root = temp_workspace("galfus_reserved_builtin_std_io_ok");
    let src = root.join("src");

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&src)?;

    write_galfus_toml(&root)?;

    fs::write(
        src.join("main.gfs"),
        r#"
import io from "std/io"

export fn main(args: [[uint8]]): int32 {
  io::print("ok")
  return 0
}
"#,
    )?;

    let check_result = check_workspace(&root)?;
    assert!(
        !check_result.has_errors(),
        "expected public std/io surface to remain usable: {:?}",
        check_result.check_result().diagnostics()
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

fn write_galfus_toml(root: &PathBuf) -> anyhow::Result<()> {
    fs::write(
        root.join("galfus.toml"),
        r#"
[module]
name = "reserved-builtins"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    Ok(())
}

fn temp_workspace(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
}
