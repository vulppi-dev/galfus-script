use anyhow::Result;
use galfus_host::Providers;
use galfus_workspace::{LoadResult, Workspace};

mod buffer_io;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(test)]
mod tests;

pub use buffer_io::BufferIoProvider;

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
    let provider = BufferIoProvider::default();
    let output_provider = provider.clone();
    let mut workspace = Workspace::new();

    match workspace
        .load_config(PLAYGROUND_CONFIG.as_bytes())
        .map_err(|error| anyhow::anyhow!("playground configuration error: {error:?}"))?
    {
        LoadResult::Success => {}
        LoadResult::Diagnostics(diagnostics) => {
            return Err(anyhow::anyhow!(
                "playground configuration diagnostics: {diagnostics:?}"
            ));
        }
    }
    workspace
        .load_module("src/main.gfs", code.as_bytes())
        .map_err(|error| anyhow::anyhow!("playground source error: {error:?}"))?;

    let check = workspace.check();
    if !check.is_valid {
        return Ok(PlaygroundResult {
            output: String::new(),
            exit_code: 1,
            error: Some(format!("{:?}", check.diagnostics)),
        });
    }

    let args_bytes = args
        .iter()
        .map(|arg| arg.as_bytes().to_vec())
        .collect::<Vec<_>>();
    workspace
        .compile()
        .map_err(|error| anyhow::anyhow!("playground compilation failed: {error:?}"))?;
    let exit_code = workspace
        .run(
            args_bytes.as_slice(),
            Some(Providers::with_io(Box::new(provider))),
        )
        .map_err(|error| anyhow::anyhow!("playground execution failed: {error:?}"))?
        .exit_code;

    let output = String::from_utf8_lossy(output_provider.take_output().as_slice()).into_owned();

    Ok(PlaygroundResult {
        output,
        exit_code,
        error: None,
    })
}

const PLAYGROUND_CONFIG: &str =
    "[module]\nname = \"playground\"\ntarget = \"app\"\nentry = \"src/main.gfs\"\n";
