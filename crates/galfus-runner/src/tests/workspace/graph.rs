use super::*;

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

var value: i32 = add(true, 2)

fn main(): null {
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "src/math.gfs",
        r#"
export fn add(a: i32, b: i32): i32 {
  return a
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::TypeMismatch.as_code()
            && diagnostic.message().contains("expected `i32`, got `bool`")
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
  id: i64,
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
