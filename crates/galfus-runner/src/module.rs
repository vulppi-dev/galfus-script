use anyhow::Result;
use std::path::{Path, PathBuf};


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
        if galfus_builtins::is_builtin_module(source) {
            return Ok(Some(ModuleSource::Builtin {
                name: source.to_string(),
            }));
        }
        Ok(None)
    }

    fn read(&self, source: &ModuleSource) -> Result<String> {
        match source {
            ModuleSource::Builtin { name } => galfus_builtins::BUILTIN_MODULES
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, src)| src.to_string())
                .ok_or_else(|| anyhow::anyhow!("unknown builtin `{name}`")),
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

pub fn normalize_existing_path(path: &Path) -> Result<PathBuf> {
    Ok(path.canonicalize()?)
}
