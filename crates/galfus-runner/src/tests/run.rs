use crate::run_project;
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_workspace() -> Result<PathBuf> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::current_dir()?.join(format!(".tmp/galfus-run-test-{unique}"));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn write_file(root: &Path, name: &str, text: &str) -> Result<PathBuf> {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, text)?;
    Ok(path)
}

#[test]
fn test_run_single_file_success() -> Result<()> {
    let root = temp_workspace()?;
    let file_path = write_file(
        &root,
        "main.gfs",
        r#"
        fn main(): int32 {
            var x = 10
            var y = 20
            return x + y
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap());
    if let Err(ref e) = result {
        println!("test_run_single_file_success failed: {:?}", e);
    }
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_workspace_success() -> Result<()> {
    let root = temp_workspace()?;
    write_file(
        &root,
        "galfus.toml",
        r#"
        [module]
        name = "my-app"
        target = "app"
        entry = "src/main.gfs"
        "#,
    )?;

    write_file(
        &root,
        "src/main.gfs",
        r#"
        import math from "./math"

        fn main(): int32 {
            return math::add(5, 7)
        }
        "#,
    )?;

    write_file(
        &root,
        "src/math.gfs",
        r#"
        export fn add(a: int32, b: int32): int32 {
            return a + b
        }
        "#,
    )?;

    let result = run_project(root.to_str().unwrap());
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_validation_failure() -> Result<()> {
    let root = temp_workspace()?;
    let file_path = write_file(
        &root,
        "invalid.gfs",
        r#"
        fn main(): int32 {
            return "not an integer"
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap());
    assert!(result.is_err());
    let err_msg = result.err().unwrap().to_string();
    assert!(err_msg.contains("validation failed"));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_vm_panic() -> Result<()> {
    let root = temp_workspace()?;
    let file_path = write_file(
        &root,
        "panic.gfs",
        r#"
        fn cause_panic(): int32 {
            return 10 / 0
        }

        fn main(): int32 {
            return cause_panic()
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap());
    assert!(result.is_err());
    let err_msg = result.err().unwrap().to_string();
    assert!(err_msg.contains("cause_panic"));
    assert!(err_msg.contains("VM Panic"));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_std_io_print() -> Result<()> {
    let root = temp_workspace()?;
    let file_path = write_file(
        &root,
        "main.gfs",
        r#"
        import { print } from 'std/io'

        fn main(): null {
            print('Hello from test!')
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap());
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}
