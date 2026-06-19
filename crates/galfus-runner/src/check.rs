use crate::{CheckDiagnosticCode, normalize_existing_path, print_check_result};
use anyhow::Result;
use galfus_core::{Diagnostic, DiagnosticBag, SourceFile, SourceId};
use galfus_frontend::{ImportKind, ModuleGraph, parse, resolve};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct CheckedModule {
    path: PathBuf,
    source: SourceFile,
    graph: ModuleGraph,
}

impl CheckedModule {
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn source(&self) -> &SourceFile {
        &self.source
    }

    pub fn graph(&self) -> &ModuleGraph {
        &self.graph
    }
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub modules: Vec<CheckedModule>,
    pub diagnostics: DiagnosticBag,
}

impl CheckResult {
    pub fn modules(&self) -> &[CheckedModule] {
        self.modules.as_slice()
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        &self.diagnostics
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    pub fn source_for(&self, source_id: SourceId) -> Option<&SourceFile> {
        self.modules
            .iter()
            .find(|module| module.source().id() == source_id)
            .map(CheckedModule::source)
    }
}

#[derive(Debug, Default)]
pub(crate) struct ModuleLoader {
    pub(crate) modules: Vec<CheckedModule>,
    module_by_path: HashMap<PathBuf, usize>,
    loading: HashSet<PathBuf>,
    pub(crate) diagnostics: DiagnosticBag,
}

impl ModuleLoader {
    fn check_entry(&mut self, path: &Path) -> Result<()> {
        let path = normalize_existing_path(path)?;

        self.load_module(path)?;
        self.validate_imports();

        Ok(())
    }

    pub(crate) fn load_module(&mut self, path: PathBuf) -> Result<usize> {
        if let Some(module) = self.module_by_path.get(path.as_path()).copied() {
            return Ok(module);
        }

        if self.loading.contains(path.as_path()) {
            return Ok(self.modules.len());
        }

        self.loading.insert(path.clone());

        let source_id = SourceId::new(self.modules.len() as u32);
        let text = fs::read_to_string(path.as_path())?;
        let source = SourceFile::new(source_id, path.display().to_string(), text);

        let parse_result = parse(&source);
        let resolve_result = resolve(&source, parse_result.into_graph());
        let graph = resolve_result.into_graph();

        self.diagnostics.extend(graph.diagnostics().iter().cloned());

        let module_index = self.modules.len();
        self.modules.push(CheckedModule {
            path: path.clone(),
            source,
            graph,
        });
        self.module_by_path.insert(path.clone(), module_index);

        self.load_relative_imports(module_index)?;

        self.loading.remove(path.as_path());

        Ok(module_index)
    }

    fn load_relative_imports(&mut self, module_index: usize) -> Result<()> {
        let imports = self.import_sources(module_index);

        for (source, source_node) in imports {
            if !is_relative_import(source.as_str()) {
                continue;
            }

            let path = resolve_relative_import(self.modules[module_index].path(), source.as_str());

            if !path.exists() {
                let span = self.modules[module_index]
                    .graph()
                    .syntax()
                    .node(source_node)
                    .map(|node| node.span())
                    .unwrap_or_else(|| self.modules[module_index].source().span());

                self.diagnostics.push(Diagnostic::error_with_message(
                    CheckDiagnosticCode::ImportModuleNotFound,
                    format!("import module `{source}` not found"),
                    span,
                ));
                continue;
            }

            let path = normalize_existing_path(path.as_path())?;
            self.load_module(path)?;
        }

        Ok(())
    }

    fn import_sources(&self, module_index: usize) -> Vec<(String, galfus_core::NodeId)> {
        let Some(resolution) = self.modules[module_index].graph().resolution() else {
            return Vec::new();
        };

        resolution
            .imports()
            .iter()
            .map(|import| (import.source().to_string(), import.source_node()))
            .collect()
    }

    pub(crate) fn validate_imports(&mut self) {
        for module_index in 0..self.modules.len() {
            let imports = self.module_imports(module_index);

            for import in imports {
                if import.kind != ImportKind::Named || !is_relative_import(import.source.as_str()) {
                    continue;
                }

                let path = resolve_relative_import(
                    self.modules[module_index].path(),
                    import.source.as_str(),
                );

                let Ok(path) = normalize_existing_path(path.as_path()) else {
                    continue;
                };

                let Some(target_index) = self.module_by_path.get(path.as_path()).copied() else {
                    continue;
                };

                let Some(target_resolution) = self.modules[target_index].graph().resolution()
                else {
                    continue;
                };

                let Some(imported_name) = import.imported_name else {
                    continue;
                };

                if target_resolution
                    .export_by_name(imported_name.as_str())
                    .is_some()
                {
                    continue;
                }

                let span = self.modules[module_index]
                    .graph()
                    .syntax()
                    .node(import.declaration)
                    .map(|node| node.span())
                    .unwrap_or_else(|| self.modules[module_index].source().span());

                self.diagnostics.push(Diagnostic::error_with_message(
                    CheckDiagnosticCode::MissingExport,
                    format!(
                        "module `{}` does not export `{}`",
                        import.source, imported_name
                    ),
                    span,
                ));
            }
        }
    }

    fn module_imports(&self, module_index: usize) -> Vec<ImportCheckRecord> {
        let Some(resolution) = self.modules[module_index].graph().resolution() else {
            return Vec::new();
        };

        resolution
            .imports()
            .iter()
            .map(|import| ImportCheckRecord {
                kind: import.kind(),
                source: import.source().to_string(),
                imported_name: import.imported_name().map(str::to_string),
                declaration: import.declaration(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
struct ImportCheckRecord {
    kind: ImportKind,
    source: String,
    imported_name: Option<String>,
    declaration: galfus_core::NodeId,
}

fn check_path(path: impl AsRef<Path>) -> Result<CheckResult> {
    let mut loader = ModuleLoader::default();

    loader.check_entry(path.as_ref())?;

    Ok(CheckResult {
        modules: loader.modules,
        diagnostics: loader.diagnostics,
    })
}

pub fn check_file(path: &str) -> Result<()> {
    let result = check_path(path)?;
    print_check_result(&result);
    Ok(())
}

fn is_relative_import(source: &str) -> bool {
    source.starts_with("./") || source.starts_with("../")
}

fn resolve_relative_import(base_module: &Path, source: &str) -> PathBuf {
    let base_dir = base_module.parent().unwrap_or_else(|| Path::new(""));
    let mut path = base_dir.join(source);

    if path.extension().is_none() {
        path.set_extension("gfs");
    }

    path
}

#[cfg(test)]
mod tests {
    use galfus_core::DiagnosticCodeKind;

    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project() -> Result<PathBuf> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("galfus-runner-test-{unique}"));

        fs::create_dir_all(path.as_path())?;

        Ok(path)
    }

    fn write_file(root: &Path, name: &str, text: &str) -> Result<PathBuf> {
        let path = root.join(name);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path.as_path(), text)?;

        Ok(path)
    }

    #[test]
    fn check_path_loads_relative_imported_modules() -> Result<()> {
        let root = temp_project()?;
        let main = write_file(
            root.as_path(),
            "main.gfs",
            r#"
            import user from "./user"

            fn main(): null {
                return
            }
            "#,
        )?;

        write_file(
            root.as_path(),
            "user.gfs",
            r#"
            export fn create(): null {
                return
            }
            "#,
        )?;

        let result = check_path(main.as_path())?;

        assert!(!result.has_errors());
        assert_eq!(result.modules().len(), 2);

        fs::remove_dir_all(root)?;

        Ok(())
    }

    #[test]
    fn check_path_accepts_named_import_from_exported_symbol() -> Result<()> {
        let root = temp_project()?;
        let main = write_file(
            root.as_path(),
            "main.gfs",
            r#"
            import { User } from "./user"

            fn main(value: User): null {
                return
            }
            "#,
        )?;

        write_file(
            root.as_path(),
            "user.gfs",
            r#"
            export struct User {
                id: int64,
            }
            "#,
        )?;

        let result = check_path(main.as_path())?;

        assert!(!result.has_errors());

        fs::remove_dir_all(root)?;

        Ok(())
    }

    #[test]
    fn check_path_reports_named_import_from_private_symbol() -> Result<()> {
        let root = temp_project()?;
        let main = write_file(
            root.as_path(),
            "main.gfs",
            r#"
            import { User } from "./user"

            fn main(value: User): null {
                return
            }
            "#,
        )?;

        write_file(
            root.as_path(),
            "user.gfs",
            r#"
            struct User {
                id: int64,
            }
            "#,
        )?;

        let result = check_path(main.as_path())?;

        assert!(result.has_errors());
        assert!(result.diagnostics().iter().any(|diagnostic| {
            diagnostic.code().as_str() == CheckDiagnosticCode::MissingExport.as_code()
                && diagnostic.message().contains("does not export `User`")
        }));

        fs::remove_dir_all(root)?;

        Ok(())
    }

    #[test]
    fn check_path_reports_missing_relative_import_module() -> Result<()> {
        let root = temp_project()?;
        let main = write_file(
            root.as_path(),
            "main.gfs",
            r#"
            import missing from "./missing"

            fn main(): null {
                return
            }
            "#,
        )?;

        let result = check_path(main.as_path())?;

        assert!(result.has_errors());
        assert!(result.diagnostics().iter().any(|diagnostic| {
            diagnostic.code().as_str() == CheckDiagnosticCode::ImportModuleNotFound.as_code()
                && diagnostic.message().contains("not found")
        }));

        fs::remove_dir_all(root)?;

        Ok(())
    }
}
