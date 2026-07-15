use std::path::Path;

use anyhow::Result;

use crate::workspace::execute_workspace;

pub fn run_project(path: &str, cli_args: &[String]) -> Result<()> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: `{}`", path.display()));
    }

    if !path.is_dir() {
        return Err(anyhow::anyhow!(
            "Running a source file directly is not supported; pass its workspace directory"
        ));
    }

    let args_bytes = cli_args
        .iter()
        .map(|argument| argument.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let exit_code = execute_workspace(path, args_bytes.as_slice())?;
    println!("Program exited successfully with code: {exit_code}");

    Ok(())
}
