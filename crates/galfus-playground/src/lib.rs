use anyhow::{Context, Result};
use galfus_target::WebTarget;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(test)]
mod tests;

pub struct PlaygroundResult {
    pub output: String,
    pub exit_code: i32,
    pub error: Option<String>,
}

pub fn run_source(code: &str, args: &[&str]) -> PlaygroundResult {
    match run_source_inner(code, args) {
        Ok(result) => result,
        Err(error) => PlaygroundResult {
            output: String::new(),
            exit_code: 1,
            error: Some(error.to_string()),
        },
    }
}

fn run_source_inner(code: &str, args: &[&str]) -> Result<PlaygroundResult> {
    let workspace = PlaygroundWorkspace::create(code)?;
    let check_result = galfus_runner::check_workspace(workspace.root())?;

    if check_result.has_errors() {
        return Ok(PlaygroundResult {
            output: String::new(),
            exit_code: 1,
            error: Some(format!("{:?}", check_result.diagnostics())),
        });
    }

    let module_image = galfus_runner::compile_workspace_to_image(&check_result)?;
    let module_name = module_image.name.clone();

    let target = WebTarget::new();
    let output_target = target.clone();
    let mut runtime = galfus_runtime::Runtime::new(Box::new(target));
    runtime.loader().load(module_image);

    let args_bytes = args
        .iter()
        .map(|arg| arg.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let exit_code = runtime
        .run_entry(
            module_name.as_str(),
            check_result.run_entry(),
            args_bytes.as_slice(),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    let output = String::from_utf8_lossy(output_target.take_output().as_slice()).into_owned();

    Ok(PlaygroundResult {
        output,
        exit_code,
        error: None,
    })
}

struct PlaygroundWorkspace {
    root: PathBuf,
}

impl PlaygroundWorkspace {
    fn create(code: &str) -> Result<Self> {
        let root = std::env::temp_dir().join(format!(
            "galfus-playground-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("system clock is before UNIX_EPOCH")?
                .as_nanos()
        ));
        let src = root.join("src");
        fs::create_dir_all(src.as_path())?;
        fs::write(
            root.join("galfus.toml"),
            "[module]\nname = \"playground\"\ntarget = \"app\"\nentry = \"src/main.gfs\"\n",
        )?;
        fs::write(src.join("main.gfs"), code)?;

        Ok(Self { root })
    }

    fn root(&self) -> &Path {
        self.root.as_path()
    }
}

impl Drop for PlaygroundWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.root.as_path());
    }
}
