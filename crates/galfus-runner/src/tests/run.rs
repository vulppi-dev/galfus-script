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
        export fn main(args: [[uint8]]): int32 {
            var x = 10
            var y = 20
            return x + y
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap(), &[]);
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

        export fn main(args: [[uint8]]): int32 {
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

    let result = run_project(root.to_str().unwrap(), &[]);
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_workspace_custom_entry_success() -> Result<()> {
    let root = temp_workspace()?;
    write_file(
        &root,
        "galfus.toml",
        r#"
        [module]
        name = "my-app"
        target = "app"
        entry = "src/main.gfs"

        [run]
        entry = "start"
        "#,
    )?;

    write_file(
        &root,
        "src/main.gfs",
        r#"
        export fn start(args: [[uint8]]): int32 {
            return 9
        }
        "#,
    )?;

    let result = run_project(root.to_str().unwrap(), &[]);
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_requires_exported_entry() -> Result<()> {
    let root = temp_workspace()?;
    let file_path = write_file(
        &root,
        "main.gfs",
        r#"
        fn main(args: [[uint8]]): int32 {
            return 0
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap(), &[]);
    assert!(result.is_err());
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("is not exported")
    );

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
        export fn main(args: [[uint8]]): int32 {
            return "not an integer"
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap(), &[]);
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

        export fn main(args: [[uint8]]): int32 {
            return cause_panic()
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap(), &[]);
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

        export fn main(args: [[uint8]]): int32 {
            print('Hello from test!')
            return 0
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap(), &[]);
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_format_stringify_core_values() -> Result<()> {
    let root = temp_workspace()?;
    let file_path = write_file(
        &root,
        "main.gfs",
        r#"
        import { println } from 'std/io'
        import { stringify } from 'format'

        export fn main(args: [[uint8]]): int32 {
            println(stringify(2548))
            println(stringify(10.42))
            println(stringify(true))
            println(stringify(null))
            return 0
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap(), &[]);
    if let Err(ref e) = result {
        println!("test_run_format_stringify_core_values failed: {:?}", e);
    }
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_buffer_create_with_runtime_length() -> Result<()> {
    let root = temp_workspace()?;
    let file_path = write_file(
        &root,
        "main.gfs",
        r#"
        fn make(n: int32): [uint8] {
            return new([uint8], n)
        }

        export fn main(args: [[uint8]]): int32 {
            var bytes = make(5)
            if bytes.length != 5 {
                return 1 / 0
            }
            return 0
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap(), &[]);
    if let Err(ref e) = result {
        println!("test_run_buffer_create_with_runtime_length failed: {:?}", e);
    }
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_buffer_create_allows_index_assignment() -> Result<()> {
    let root = temp_workspace()?;
    let file_path = write_file(
        &root,
        "main.gfs",
        r#"
        export fn main(args: [[uint8]]): int32 {
            var values = new([int32], 3)
            values[0] = 10
            values[1] = 20
            values[2] = 30
            if values[0] != 10 || values[1] != 20 || values[2] != 30 {
                return 1 / 0
            }
            return 0
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap(), &[]);
    if let Err(ref e) = result {
        println!(
            "test_run_buffer_create_allows_index_assignment failed: {:?}",
            e
        );
    }
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_workspace_with_args() -> Result<()> {
    let root = temp_workspace()?;
    write_file(
        &root,
        "galfus.toml",
        r#"
        [module]
        name = "my-app"
        target = "app"
        entry = "src/main.gfs"

        [run]
        args = ["conf"]
        "#,
    )?;

    write_file(
        &root,
        "src/main.gfs",
        r#"
        import { println } from 'std/io'

        export fn main(args: [[uint8]]): int32 {
            for v in args {
                println(v)
            }
            return 0
        }
        "#,
    )?;

    let result = run_project(root.to_str().unwrap(), &["prop".to_string()]);
    if let Err(ref e) = result {
        println!("test_run_workspace_with_args failed: {:?}", e);
    }
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_run_ansi_apply_with_stringify_string() -> Result<()> {
    let root = temp_workspace()?;
    let file_path = write_file(
        &root,
        "main.gfs",
        r#"
        import { println } from 'std/io'
        import { stringify } from 'format'
        import { red } from 'format/ansi'

        export fn main(args: [[uint8]]): int32 {
            println(red()::apply(stringify("Hello")))
            return 0
        }
        "#,
    )?;

    let result = run_project(file_path.to_str().unwrap(), &[]);
    if let Err(ref e) = result {
        println!("test_run_ansi_apply_with_stringify_string failed: {:?}", e);
    }
    assert!(result.is_ok());

    fs::remove_dir_all(root)?;
    Ok(())
}
