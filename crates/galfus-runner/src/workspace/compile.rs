use anyhow::Result;
use galfus_compiler::{CompiledModule, CompilerInput};
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
    // The compiler uses ModulePath (host-agnostic, normalized) as module identifiers.
    // We relativize native paths against CWD so cross-module import resolution
    // in the compiler can reconstruct relative import edges.
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut compiled_modules: Vec<CompiledModule> = runner_modules
        .iter()
        .map(|m| {
            let native_path = m.path();
            let relative = native_path.strip_prefix(&cwd).unwrap_or(native_path);
            let path_str = relative.to_string_lossy().replace('\\', "/");
            let module_path = galfus_core::ModulePath::new(&path_str)
                .or_else(|| galfus_core::ModulePath::new(format!("{path_str}.gfs").as_str()))
                .unwrap_or_else(|| galfus_core::ModulePath::new("unknown.gfs").unwrap());
            CompiledModule::new(
                module_path,
                m.source().clone(),
                m.graph().clone(),
                m.type_result().cloned(),
            )
        })
        .collect();

    // Determine the entry module index using the already-converted ModulePath list.
    let entry_native = check_result
        .graph()
        .roots()
        .iter()
        .find(|r| matches!(r.kind(), WorkspaceRootKind::Entry))
        .map(|r| r.path().to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("No entrypoint defined in workspace config"))?;

    let entry_relative = entry_native
        .strip_prefix(&cwd)
        .unwrap_or(&entry_native)
        .to_string_lossy()
        .replace('\\', "/");
    let entry_module_path = galfus_core::ModulePath::new(&entry_relative)
        .ok_or_else(|| anyhow::anyhow!("Entry path is not a valid ModulePath: {entry_relative}"))?;

    let entry_index = compiled_modules
        .iter()
        .position(|m| m.path() == &entry_module_path)
        .ok_or_else(|| anyhow::anyhow!("Entry module not found: {entry_relative}"))?;

    let image_name = entry_module_path.as_str().to_string();

    let mut input = CompilerInput {
        modules: compiled_modules.as_mut_slice(),
        entry_index,
        image_name,
    };

    galfus_compiler::compile_to_image(&mut input)
}
