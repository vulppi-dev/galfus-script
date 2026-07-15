use galfus_core::ModulePath;

pub fn is_resolvable_import(source: &str) -> bool {
    source.starts_with("./") || source.starts_with("../") || is_builtin_module(source)
}

pub fn is_relative_import(source: &str) -> bool {
    source.starts_with("./") || source.starts_with("../")
}

pub fn is_builtin_module(source: &str) -> bool {
    // For now we check a hardcoded string prefix or we can depend on galfus_builtins.
    // In the runner it used `galfus_builtins::is_builtin_module`.
    // We should probably just do that, or handle it here if frontend doesn't depend on builtins.
    // Let's assume frontend can depend on builtins or we just check "std/".
    // Since Phase 2 hasn't fully integrated galfus_builtins into frontend, we will just use a heuristic.
    // Wait, the plan says "Implicit semantic dependencies" so it might depend on builtins.
    source.starts_with("std/")
}

// In the new architecture, the module path resolution logic belongs in the frontend,
// because it affects semantic module identity.
pub fn resolve_relative_import(base_module: &ModulePath, source: &str) -> Option<ModulePath> {
    // Example: base_module = "src/main.gfs", source = "./utils" -> "src/utils.gfs"
    let base_str = base_module.as_str();
    let mut segments: Vec<&str> = base_str.split('/').collect();

    // Remove the file part
    if !segments.is_empty() {
        segments.pop();
    }

    let source_segments: Vec<&str> = source.split('/').collect();
    for segment in source_segments {
        if segment == "." || segment.is_empty() {
            continue;
        } else if segment == ".." {
            if segments.is_empty() {
                return None;
            }
            segments.pop();
        } else {
            segments.push(segment);
        }
    }

    let mut path = segments.join("/");
    if !path.ends_with(".gfs") {
        path.push_str(".gfs");
    }

    ModulePath::new(&path)
}
