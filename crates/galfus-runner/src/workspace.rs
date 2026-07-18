use anyhow::{Context, Result, bail};
use galfus_workspace::{LoadResult, Workspace};
use std::path::Path;

#[cfg(test)]
mod tests;

pub fn check_workspace_root(root: &str) -> Result<()> {
    let mut workspace = load_workspace(Path::new(root))?;
    let report = workspace.check();
    for diagnostic in report.diagnostics.iter() {
        println!(
            "{:?} {}: {}",
            diagnostic.severity(),
            diagnostic.code().as_str(),
            diagnostic.message()
        );
    }
    if report.is_valid {
        Ok(())
    } else {
        bail!("workspace validation failed")
    }
}

pub fn run_project(root: &str, cli_args: &[String]) -> Result<()> {
    let mut workspace = load_workspace(Path::new(root))?;
    let report = workspace.check();
    if !report.is_valid {
        bail!("workspace validation failed: {:?}", report.diagnostics);
    }
    workspace
        .compile()
        .map_err(|error| anyhow::anyhow!("workspace compilation failed: {error:?}"))?;
    let args = cli_args
        .iter()
        .map(|argument| argument.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let report = workspace
        .run(args.as_slice())
        .map_err(|error| anyhow::anyhow!("workspace execution failed: {error:?}"))?;
    println!(
        "Program exited successfully with code: {}",
        report.exit_code
    );
    Ok(())
}

fn load_workspace(root: &Path) -> Result<Workspace> {
    let root = if root.is_file() {
        root.parent().unwrap_or_else(|| Path::new(""))
    } else {
        root
    }
    .canonicalize()
    .context("workspace root does not exist")?;
    let config = std::fs::read(root.join("galfus.toml"))?;

    let mut workspace = Workspace::new();
    if let LoadResult::Diagnostics(diagnostics) = workspace
        .load_config(config.as_slice())
        .map_err(|error| anyhow::anyhow!("workspace configuration error: {error:?}"))?
    {
        bail!("workspace configuration failed: {diagnostics:?}");
    }

    load_sources(&mut workspace, root.as_path(), root.as_path())?;
    Ok(workspace)
}

fn load_sources(workspace: &mut Workspace, workspace_root: &Path, directory: &Path) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            load_sources(workspace, workspace_root, path.as_path())?;
            continue;
        }
        if !file_type.is_file() || path.extension().is_none_or(|extension| extension != "gfs") {
            continue;
        }

        let source = std::fs::read(path.as_path())?;
        let module_path = path
            .strip_prefix(workspace_root)
            .context("source module is outside the workspace root")?;
        let module_path = module_path.to_string_lossy().replace('\\', "/");
        workspace
            .load_module(module_path.as_str(), source.as_slice())
            .map_err(|error| anyhow::anyhow!("workspace source error: {error:?}"))?;
    }
    Ok(())
}
