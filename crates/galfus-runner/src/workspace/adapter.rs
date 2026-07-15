use anyhow::Result;
use galfus_compiler::CompiledModuleGraph;
use galfus_core::{DiagnosticBag, ModulePath};
use galfus_workspace::{LoadResult, Workspace};
use std::path::Path;
use std::sync::Arc;

use crate::check::ModuleLoader;
use crate::module::normalize_existing_path;

use super::config::parse_workspace_config;

/// Disk and builtin source adapter for the stateful workspace façade.
///
/// `ModuleLoader` is used here only to discover the complete import closure.
/// Semantic checking is performed by `galfus_workspace::Workspace`.
pub fn load_workspace_for_check(root: impl AsRef<Path>) -> Result<Workspace> {
    let root = root.as_ref();
    let root = if root.is_file() {
        root.parent().unwrap_or_else(|| Path::new(""))
    } else {
        root
    };
    let root = normalize_existing_path(root)?;
    let config_path = root.join("galfus.toml");
    let config_text = std::fs::read(config_path)?;

    let mut workspace = Workspace::new();
    match workspace
        .load_config(config_text.as_slice())
        .map_err(|error| anyhow::anyhow!("workspace configuration error: {error:?}"))?
    {
        LoadResult::Success => {}
        LoadResult::Diagnostics(_) => return Ok(workspace),
    }

    let mut diagnostics = DiagnosticBag::new();
    let config = parse_workspace_config(
        root.as_path(),
        std::str::from_utf8(config_text.as_slice())?,
        &mut diagnostics,
    );
    if diagnostics.has_errors() {
        return Ok(workspace);
    }
    let Some(config) = config else {
        return Ok(workspace);
    };

    let mut loader = ModuleLoader::default();
    if let Some(entry) = config.entry() {
        loader.load_module(normalize_existing_path(entry)?)?;
    }
    for export in config.exports() {
        loader.load_module(normalize_existing_path(export.path())?)?;
    }

    for module in &loader.modules {
        let path = workspace_path(root.as_path(), module.path())?;
        workspace
            .load_module(path.as_str(), module.source().text().as_bytes())
            .map_err(|error| anyhow::anyhow!("workspace source error: {error:?}"))?;
    }

    Ok(workspace)
}

/// Compile a workspace through the stateful workspace façade.
pub fn compile_workspace_modules(root: impl AsRef<Path>) -> Result<Arc<CompiledModuleGraph>> {
    let mut workspace = load_workspace_for_check(root)?;
    let check = workspace.check();
    if !check.is_valid {
        return Err(anyhow::anyhow!(
            "workspace validation failed: {:?}",
            check.diagnostics
        ));
    }
    let report = workspace
        .compile()
        .map_err(|error| anyhow::anyhow!("workspace compilation failed: {error:?}"))?;
    Ok(report.graph)
}

/// Execute a workspace through the stateful workspace façade.
pub fn execute_workspace(root: impl AsRef<Path>, args: &[Vec<u8>]) -> Result<i32> {
    let mut workspace = load_workspace_for_check(root)?;
    let check = workspace.check();
    if !check.is_valid {
        return Err(anyhow::anyhow!(
            "workspace validation failed: {:?}",
            check.diagnostics
        ));
    }
    workspace
        .compile()
        .map_err(|error| anyhow::anyhow!("workspace compilation failed: {error:?}"))?;
    workspace
        .run(args)
        .map(|report| report.exit_code)
        .map_err(|error| anyhow::anyhow!("workspace execution failed: {error:?}"))
}

fn workspace_path(root: &Path, path: &Path) -> Result<ModulePath> {
    let value = if galfus_builtins::is_builtin_module(path.to_string_lossy().as_ref()) {
        format!("{}.gfs", path.display())
    } else {
        path.strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    };
    ModulePath::new(value.as_str())
        .ok_or_else(|| anyhow::anyhow!("unsupported module path `{}`", path.display()))
}
