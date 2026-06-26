use galfus_core::DiagnosticCodeKind;
use galfus_frontend::{ImportKind, TypeDiagnosticCode};

use crate::{WorkspaceDiagnosticCode, workspace::check_workspace};
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_workspace() -> Result<PathBuf> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!("galfus-workspace-test-{unique}"));
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
fn check_workspace_accepts_valid_app_entry() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
fn main(): null {
  return
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.modules().len(), 1);

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_reports_missing_galfus_toml() -> Result<()> {
    let root = temp_workspace()?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == WorkspaceDiagnosticCode::MissingConfig.as_code()
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_reports_app_without_entry() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == WorkspaceDiagnosticCode::MissingAppEntry.as_code()
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_reports_lib_without_entry_or_exports() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-lib"
target = "lib"
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == WorkspaceDiagnosticCode::MissingLibrarySurface.as_code()
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_accepts_lib_with_exports() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-lib"
target = "lib"

[exports]
"user" = "src/user.gfs"
"result" = "src/result.gfs"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/user.gfs",
        r#"
export struct User {
  id: int64,
}
"#,
    )?;

    write_file(
        root.as_path(),
        "src/result.gfs",
        r#"
export choice Result<V, E> {
  Ok(V),
  Err(E),
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.modules().len(), 2);

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_reports_missing_entry_target() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/missing.gfs"
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == WorkspaceDiagnosticCode::EntryTargetMissing.as_code()
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_reports_export_target_missing() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-lib"
target = "lib"

[exports]
"user" = "src/missing.gfs"
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == WorkspaceDiagnosticCode::ExportTargetMissing.as_code()
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_rejects_non_gfs_entry() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.toml"
"#,
    )?;

    write_file(root.as_path(), "src/main.toml", "")?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == WorkspaceDiagnosticCode::UnsupportedWorkspaceTarget.as_code()
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_builds_graph_with_entry_root() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
fn main(): null {
  return
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.graph().modules().len(), 1);
    assert_eq!(result.graph().roots().len(), 1);
    assert_eq!(result.graph().import_edges().len(), 0);

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_builds_graph_with_relative_import_edge() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
import user from "./user"

fn main(): null {
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "src/user.gfs",
        r#"
export fn create(): null {
  return
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.graph().modules().len(), 2);
    assert_eq!(result.graph().roots().len(), 1);
    assert_eq!(result.graph().import_edges().len(), 1);
    assert!(result.graph().import_edges()[0].is_resolved());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_resolves_named_import_export_edge() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
import { create as make } from "./user"

fn main(): null {
  make()
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "src/user.gfs",
        r#"
export fn create(): null {
  return
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.graph().import_edges().len(), 1);

    let edge = &result.graph().import_edges()[0];
    assert_eq!(edge.kind(), ImportKind::Named);
    assert_eq!(edge.local_name(), "make");
    assert_eq!(edge.imported_name(), Some("create"));
    assert_eq!(edge.export_name(), Some("create"));
    assert!(edge.is_export_resolved());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_reports_named_imported_function_type_error() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
import { add } from "./math"

var value: int32 = add(true, 2)

fn main(): null {
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "src/math.gfs",
        r#"
export fn add(a: int32, b: int32): int32 {
  return a
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::TypeMismatch.as_code()
            && diagnostic
                .message()
                .contains("expected `int32`, got `bool`")
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_resolves_namespace_import_export_references() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
import user from "./user"

fn main(value: user::User): null {
  user::create()
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "src/user.gfs",
        r#"
export struct User {
  id: int64,
}

export fn create(): null {
  return
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.graph().import_edges().len(), 1);

    let edge = &result.graph().import_edges()[0];
    assert_eq!(edge.kind(), ImportKind::Namespace);
    assert_eq!(edge.local_name(), "user");
    assert_eq!(
        edge.referenced_exports(),
        &vec!["User".to_string(), "create".to_string()]
    );

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_builds_graph_for_import_cycle() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
import user from "./user"

fn main(): null {
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "src/user.gfs",
        r#"
import main from "./main"

export fn create(): null {
  return
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.graph().modules().len(), 2);
    assert_eq!(result.graph().roots().len(), 1);
    assert_eq!(result.graph().import_edges().len(), 2);
    assert!(
        result
            .graph()
            .import_edges()
            .iter()
            .all(|edge| edge.is_resolved())
    );

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_workspace_reports_namespace_import_from_private_export() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
import user from "./user"

fn main(): null {
  user::create()
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "src/user.gfs",
        r#"
fn create(): null {
  return
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == WorkspaceDiagnosticCode::ExportTargetMissing.as_code()
            || diagnostic.message().contains("does not export `create`")
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}
