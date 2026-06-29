use super::*;

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
fn check_workspace_rejects_qualified_run_entry() -> Result<()> {
    let root = temp_workspace()?;

    write_file(
        root.as_path(),
        "galfus.toml",
        r#"
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"

[run]
entry = "app.start"
"#,
    )?;

    write_file(
        root.as_path(),
        "src/main.gfs",
        r#"
export fn start(args: [[uint8]]): int32 {
  return 0
}
"#,
    )?;

    let result = check_workspace(root.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == WorkspaceDiagnosticCode::InvalidConfig.as_code()
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}
