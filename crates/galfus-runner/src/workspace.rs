use anyhow::{Context, Result, bail};
use galfus_core::{SourceFile, SourceId};
use galfus_frontend::{parse, resolve};
use galfus_workspace::{LoadResult, Workspace};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
    let entries = workspace_roots(config.as_slice())?;

    let mut workspace = Workspace::new();
    if let LoadResult::Diagnostics(diagnostics) = workspace
        .load_config(config.as_slice())
        .map_err(|error| anyhow::anyhow!("workspace configuration error: {error:?}"))?
    {
        bail!("workspace configuration failed: {diagnostics:?}");
    }

    let mut loaded = HashSet::new();
    for entry in entries {
        load_source_closure(&mut workspace, root.as_path(), entry.as_path(), &mut loaded)?;
    }
    Ok(workspace)
}

fn workspace_roots(config: &[u8]) -> Result<Vec<PathBuf>> {
    let config = std::str::from_utf8(config).context("galfus.toml is not UTF-8")?;
    let value = toml::from_str::<toml::Value>(config).context("invalid galfus.toml")?;
    let mut roots = value
        .get("module")
        .and_then(toml::Value::as_table)
        .and_then(|module| module.get("entry"))
        .and_then(toml::Value::as_str)
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();
    if let Some(exports) = value.get("exports").and_then(toml::Value::as_table) {
        roots.extend(
            exports
                .values()
                .filter_map(toml::Value::as_str)
                .map(PathBuf::from),
        );
    }
    if roots.is_empty() {
        bail!("workspace has no source roots")
    }
    Ok(roots)
}

fn load_source_closure(
    workspace: &mut Workspace,
    root: &Path,
    relative_path: &Path,
    loaded: &mut HashSet<PathBuf>,
) -> Result<()> {
    let path = root
        .join(relative_path)
        .canonicalize()
        .with_context(|| format!("source module `{}` does not exist", relative_path.display()))?;
    if !loaded.insert(path.clone()) {
        return Ok(());
    }
    let source = std::fs::read_to_string(path.as_path())?;
    let module_path = path
        .strip_prefix(root)
        .context("source module is outside the workspace root")?;
    let module_path = module_path.to_string_lossy().replace('\\', "/");
    workspace
        .load_module(module_path.as_str(), source.as_bytes())
        .map_err(|error| anyhow::anyhow!("workspace source error: {error:?}"))?;

    let source_file = SourceFile::new(SourceId::new(0), module_path, source);
    let graph = resolve(&source_file, parse(&source_file).into_graph()).into_graph();
    let Some(resolution) = graph.resolution() else {
        return Ok(());
    };
    for import in resolution.imports() {
        let import = import.source();
        if !import.starts_with("./") && !import.starts_with("../") {
            continue;
        }
        let mut next = relative_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(import);
        if next.extension().is_none() {
            next.set_extension("gfs");
        }
        load_source_closure(workspace, root, next.as_path(), loaded)?;
    }
    Ok(())
}
