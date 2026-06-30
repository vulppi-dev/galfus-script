#[cfg(test)]
mod tests;

mod check;
mod compile_workspace;
mod diagnostic;
mod local_graph;
mod workspace;
mod workspace_graph;

pub use check::*;
pub use compile_workspace::*;
pub use diagnostic::*;
pub use local_graph::*;
pub use workspace::*;
pub use workspace_graph::*;

use anyhow::Result;
use galfus_core::Diagnostic;
use std::path::{Path, PathBuf};

const STD_IO_MODULE: &str = "std/io";
const TEXT_MODULE: &str = "text";
const FORMAT_MODULE: &str = "format";
const FORMAT_ANSI_MODULE: &str = "format/ansi";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModuleSource {
    File(PathBuf),
    Builtin { name: String },
}

impl ModuleSource {
    pub(crate) fn path(&self) -> PathBuf {
        match self {
            Self::File(path) => path.clone(),
            Self::Builtin { name } => PathBuf::from(name),
        }
    }
}

pub(crate) trait ModuleSourceProvider {
    fn resolve(&self, base_module: &Path, source: &str) -> Result<Option<ModuleSource>>;
    fn read(&self, source: &ModuleSource) -> Result<String>;
}

#[derive(Debug)]
pub(crate) struct FileSourceProvider;

impl ModuleSourceProvider for FileSourceProvider {
    fn resolve(&self, base_module: &Path, source: &str) -> Result<Option<ModuleSource>> {
        if !source.starts_with("./") && !source.starts_with("../") {
            return Ok(None);
        }

        let base_dir = base_module.parent().unwrap_or_else(|| Path::new(""));
        let mut path = base_dir.join(source);

        if path.extension().is_none() {
            path.set_extension("gfs");
        }

        Ok(Some(ModuleSource::File(normalize_existing_path(
            path.as_path(),
        )?)))
    }

    fn read(&self, source: &ModuleSource) -> Result<String> {
        match source {
            ModuleSource::File(path) => Ok(std::fs::read_to_string(path.as_path())?),
            ModuleSource::Builtin { .. } => unreachable!("file provider received builtin source"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct BuiltinSourceProvider;

impl ModuleSourceProvider for BuiltinSourceProvider {
    fn resolve(&self, _base_module: &Path, source: &str) -> Result<Option<ModuleSource>> {
        match source {
            s if s == STD_IO_MODULE
                || s == TEXT_MODULE
                || s == FORMAT_MODULE
                || s == FORMAT_ANSI_MODULE =>
            {
                Ok(Some(ModuleSource::Builtin {
                    name: source.to_string(),
                }))
            }
            _ => Ok(None),
        }
    }

    fn read(&self, source: &ModuleSource) -> Result<String> {
        match source {
            ModuleSource::Builtin { name } if name == STD_IO_MODULE => {
                Ok(galfus_builtins::STD_IO_SOURCE.to_string())
            }
            ModuleSource::Builtin { name } if name == TEXT_MODULE => {
                Ok(galfus_builtins::TEXT_SOURCE.to_string())
            }
            ModuleSource::Builtin { name } if name == FORMAT_MODULE => {
                Ok(galfus_builtins::FORMAT_SOURCE.to_string())
            }
            ModuleSource::Builtin { name } if name == FORMAT_ANSI_MODULE => {
                Ok(galfus_builtins::FORMAT_ANSI_SOURCE.to_string())
            }
            ModuleSource::Builtin { name } => Err(anyhow::anyhow!("unknown builtin `{name}`")),
            ModuleSource::File(_) => unreachable!("builtin provider received file source"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct WorkspaceResolver {
    file: FileSourceProvider,
    builtin: BuiltinSourceProvider,
}

impl Default for WorkspaceResolver {
    fn default() -> Self {
        Self {
            file: FileSourceProvider,
            builtin: BuiltinSourceProvider,
        }
    }
}

impl WorkspaceResolver {
    pub(crate) fn resolve_import(&self, base_module: &Path, source: &str) -> Result<ModuleSource> {
        if let Some(source) = self.builtin.resolve(base_module, source)? {
            return Ok(source);
        }
        if let Some(source) = self.file.resolve(base_module, source)? {
            return Ok(source);
        }
        Err(anyhow::anyhow!("unresolvable import `{source}`"))
    }

    pub(crate) fn read(&self, source: &ModuleSource) -> Result<String> {
        match source {
            ModuleSource::File { .. } => self.file.read(source),
            ModuleSource::Builtin { .. } => self.builtin.read(source),
        }
    }
}

fn normalize_existing_path(path: &Path) -> Result<PathBuf> {
    Ok(path.canonicalize()?)
}

fn print_check_result(result: &CheckResult) {
    println!("modules: {}", result.modules().len());

    for module in result.modules() {
        println!(
            "  {:?}: {:?}, syntax nodes: {}",
            module.path(),
            module.graph().phase(),
            module.graph().syntax().len()
        );
    }

    if result.diagnostics().is_empty() {
        println!("ok");
        return;
    }

    println!("diagnostics:");

    for diagnostic in result.diagnostics().iter() {
        print_diagnostic(result, diagnostic);
    }
}

fn print_diagnostic(result: &CheckResult, diagnostic: &Diagnostic) {
    let source = result.source_for(diagnostic.span().source_id());

    if let Some(source) = source {
        let pos = source.row_col(diagnostic.span().start());

        if let Some(pos) = pos {
            println!(
                "  {:?} {} at {}:{}:{}: {}",
                diagnostic.severity(),
                diagnostic.code().as_str(),
                source.name(),
                pos.row,
                pos.column,
                diagnostic.message()
            );
            return;
        }
    }

    println!(
        "  {:?} {}: {}",
        diagnostic.severity(),
        diagnostic.code().as_str(),
        diagnostic.message()
    );
}

pub fn compile_file_to_gfb(source_path: &Path, output_path: &Path) -> Result<()> {
    use std::fs;

    let code = fs::read_to_string(source_path)?;
    let source_id = galfus_core::SourceId::new(0);
    let source_file = galfus_core::SourceFile::new(
        source_id,
        source_path.to_string_lossy().into_owned(),
        code.clone(),
    );

    let parse_result = galfus_frontend::parse(&source_file);
    let resolve_result = galfus_frontend::resolve(&source_file, parse_result.into_graph());
    let graph = resolve_result.into_graph();

    if graph.has_errors() {
        return Err(anyhow::anyhow!(
            "Compilation failed during parsing/resolution: {:?}",
            graph.diagnostics()
        ));
    }

    let type_result = galfus_frontend::check_declaration_types(&source_file, &graph);
    if type_result.has_errors() {
        return Err(anyhow::anyhow!(
            "Compilation failed during type-checking: {:?}",
            type_result.diagnostics()
        ));
    }

    let mir_module = galfus_ir::builder::MirBuilder::new(&graph, &type_result, &code).build();
    let module_image = galfus_ir::lower::lower_module(&mir_module, &type_result, &graph, &code);

    if let Err(errors) = galfus_image::validation::validate_module_image(&module_image) {
        return Err(anyhow::anyhow!(
            "ModuleImage validation failed: {:?}",
            errors
        ));
    }

    let gfb_bytes = galfus_image::gfb::serialize_to_gfb(&module_image)
        .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;

    fs::write(output_path, gfb_bytes)?;

    Ok(())
}

pub fn load_gfb_file(path: &Path) -> Result<galfus_image::ModuleImage> {
    use std::fs;

    let bytes = fs::read(path)?;
    let module_image = galfus_image::gfb::deserialize_from_gfb(&bytes)
        .map_err(|e| anyhow::anyhow!("GFB loader error: {}", e))?;
    Ok(module_image)
}

pub fn run_project(path: &str, cli_args: &[String]) -> Result<()> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: `{}`", path.display()));
    }

    let is_dir = path.is_dir();
    let (entry_path, _ws_root, tmp_dir, check_result_opt, run_entry, mut config_args) = if is_dir {
        let check_result = check_workspace(path)?;
        if check_result.has_errors() {
            print_check_result(check_result.check_result());
            return Err(anyhow::anyhow!("Workspace validation failed"));
        }

        let entry_path = check_result
            .graph()
            .roots()
            .iter()
            .find(|r| matches!(r.kind(), WorkspaceRootKind::Entry))
            .map(|r| r.path().to_path_buf())
            .ok_or_else(|| anyhow::anyhow!("No entrypoint defined in workspace config"))?;

        let ws_root = path.to_path_buf();
        let tmp_dir = ws_root.join(".tmp");
        let run_entry = check_result.run_entry().to_string();
        let run_args = check_result.run_args().to_vec();
        (
            entry_path,
            ws_root,
            tmp_dir,
            Some(check_result),
            run_entry,
            run_args,
        )
    } else {
        let check_result = check_path(path)?;
        if check_result.has_errors() {
            print_check_result(&check_result);
            return Err(anyhow::anyhow!("File validation failed"));
        }

        let entry_path = path.to_path_buf();
        let ws_root = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        let tmp_dir = ws_root.join(".tmp");
        let graph = WorkspaceGraph::for_single_file(&entry_path, check_result.modules())?;
        let ws_check_result = WorkspaceCheckResult::new(check_result, graph);
        (
            entry_path,
            ws_root,
            tmp_dir,
            Some(ws_check_result),
            "main".to_string(),
            Vec::new(),
        )
    };

    std::fs::create_dir_all(&tmp_dir)?;
    let output_path = tmp_dir.join("temp_run.gfb");

    if let Some(check_result) = check_result_opt {
        compile_workspace_to_gfb(&check_result, &output_path)?;
    } else {
        compile_file_to_gfb(&entry_path, &output_path)?;
    }

    let module_image = load_gfb_file(&output_path)?;

    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_dir(&tmp_dir);

    let module_name = module_image.name.clone();
    let mut runtime = galfus_runtime::Runtime::new(Box::new(galfus_target::NativeTarget));
    runtime.loader().load(module_image);

    config_args.extend(cli_args.to_vec());
    let args_bytes: Vec<Vec<u8>> = config_args.into_iter().map(|s| s.into_bytes()).collect();

    let exit_code = runtime
        .run_entry(module_name.as_str(), run_entry.as_str(), &args_bytes)
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    println!("Program exited successfully with code: {exit_code}");

    Ok(())
}
