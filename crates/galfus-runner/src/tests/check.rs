use galfus_core::DiagnosticCodeKind;

use crate::{CheckDiagnosticCode, check::check_path};
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_project() -> Result<PathBuf> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!("galfus-runner-test-{unique}"));

    fs::create_dir_all(path.as_path())?;

    Ok(path)
}

fn write_file(root: &Path, name: &str, text: &str) -> Result<PathBuf> {
    let path = root.join(name);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path.as_path(), text)?;

    Ok(path)
}

#[test]
fn check_path_loads_relative_imported_modules() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import user from "./user"

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
        export fn create(): null {
            return
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.modules().len(), 2);

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_accepts_named_import_from_exported_symbol() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { User } from "./user"

        fn main(value: User): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
        export struct User {
            id: int64,
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_reports_named_import_from_private_symbol() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { User } from "./user"

        fn main(value: User): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
        struct User {
            id: int64,
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == CheckDiagnosticCode::MissingExport.as_code()
            && diagnostic.message().contains("does not export `User`")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_reports_missing_relative_import_module() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import missing from "./missing"

        fn main(): null {
            return
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == CheckDiagnosticCode::ImportModuleNotFound.as_code()
            && diagnostic.message().contains("not found")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_rejects_non_gfs_relative_import_target() -> Result<()> {
    let root = temp_project()?;

    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
import data from "./data.json"

fn main(): null {
  return
}
"#,
    )?;

    write_file(root.as_path(), "data.json", "{}")?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == CheckDiagnosticCode::UnsupportedImportTarget.as_code()
            && diagnostic
                .message()
                .contains("must resolve to a .gfs source file")
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}
