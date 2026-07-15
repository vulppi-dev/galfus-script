use anyhow::Result;
use galfus_compiler::{
    CompilerInput,
    input::CompiledModule,
};
use galfus_image::ModuleImage;

use crate::workspace::{WorkspaceCheckResult, WorkspaceRootKind};

/// Compile a checked workspace into a `ModuleImage`.
///
/// This function bridges the runner's `CheckedModule` type into the
/// `galfus-compiler` crate's `CompiledModule` boundary type, then delegates
/// compilation entirely to `galfus_compiler::compile_to_image`.
pub fn compile_workspace_to_image(check_result: &WorkspaceCheckResult) -> Result<ModuleImage> {
    let runner_modules = check_result.modules();

    // Convert runner's CheckedModule to compiler's CompiledModule.
    let mut compiled_modules: Vec<CompiledModule> = runner_modules
        .iter()
        .map(|m| {
            CompiledModule::new(
                m.path(),
                m.source().clone(),
                m.graph().clone(),
                m.type_result().cloned(),
            )
        })
        .collect();

    // Determine the entry module index.
    let entry_path = check_result
        .graph()
        .roots()
        .iter()
        .find(|r| matches!(r.kind(), WorkspaceRootKind::Entry))
        .map(|r| r.path().to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("No entrypoint defined in workspace config"))?;

    let entry_index = runner_modules
        .iter()
        .position(|m| m.path() == entry_path)
        .ok_or_else(|| anyhow::anyhow!("Entry module not found in workspace checked modules"))?;

    let image_name = check_result
        .graph()
        .roots()
        .first()
        .map(|r| r.path().to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut input = CompilerInput {
        modules: compiled_modules.as_mut_slice(),
        entry_index,
        image_name,
    };

    galfus_compiler::compile_to_image(&mut input)
}
