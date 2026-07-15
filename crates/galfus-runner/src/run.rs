use std::path::Path;

use anyhow::Result;

use crate::check::check_path;
use crate::print::print_check_result;
use crate::workspace::*;

pub fn run_project(path: &str, cli_args: &[String]) -> Result<()> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: `{}`", path.display()));
    }

    let (module_image, run_entry, mut config_args) = if path.is_dir() {
        let check_result = check_workspace(path)?;
        if check_result.has_errors() {
            print_check_result(check_result.check_result());
            return Err(anyhow::anyhow!("Workspace validation failed"));
        }

        let run_entry = check_result.run_entry().to_string();
        let run_args = check_result.run_args().to_vec();
        let module_image = compile_workspace_to_image(&check_result)?;

        (module_image, run_entry, run_args)
    } else {
        let check_result = check_path(path)?;
        if check_result.has_errors() {
            print_check_result(&check_result);
            return Err(anyhow::anyhow!("File validation failed"));
        }

        let entry_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let graph = WorkspaceGraph::for_single_file(&entry_path, check_result.modules())?;
        let ws_check_result = WorkspaceCheckResult::new(check_result, graph);
        let module_image = compile_workspace_to_image(&ws_check_result)?;

        (module_image, "main".to_string(), Vec::new())
    };

    let module_name = module_image.name.clone();
    let mut runtime = galfus_runtime::Runtime::new(Box::new(galfus_target::NativeTarget));
    runtime.loader().load(module_image);

    config_args.extend_from_slice(cli_args);
    let args_bytes: Vec<Vec<u8>> = config_args.into_iter().map(|s| s.into_bytes()).collect();

    let exit_code = runtime
        .run_entry(module_name.as_str(), run_entry.as_str(), &args_bytes)
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    println!("Program exited successfully with code: {exit_code}");

    Ok(())
}
