use crate::*;
use anyhow::Result;
use galfus_core::{Diagnostic, DiagnosticBag, SourceId, Span};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const WORKSPACE_SOURCE_ID: SourceId = SourceId::new(u32::MAX);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleTarget {
    App,
    Lib,
}

impl ModuleTarget {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "app" => Some(Self::App),
            "lib" => Some(Self::Lib),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceExport {
    address: String,
    path: PathBuf,
}

impl WorkspaceExport {
    pub fn address(&self) -> &str {
        self.address.as_str()
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceConfig {
    root: PathBuf,
    name: String,
    target: ModuleTarget,
    entry: Option<PathBuf>,
    exports: Vec<WorkspaceExport>,
}

impl WorkspaceConfig {
    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn target(&self) -> ModuleTarget {
        self.target
    }

    pub fn entry(&self) -> Option<&Path> {
        self.entry.as_deref()
    }

    pub fn exports(&self) -> &[WorkspaceExport] {
        self.exports.as_slice()
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceCheckResult {
    check: CheckResult,
    graph: WorkspaceGraph,
}

impl WorkspaceCheckResult {
    pub fn new(check: CheckResult, graph: WorkspaceGraph) -> Self {
        Self { check, graph }
    }

    pub fn check_result(&self) -> &CheckResult {
        &self.check
    }

    pub fn graph(&self) -> &WorkspaceGraph {
        &self.graph
    }

    pub fn modules(&self) -> &[crate::CheckedModule] {
        self.check.modules()
    }

    pub fn diagnostics(&self) -> &galfus_core::DiagnosticBag {
        self.check.diagnostics()
    }

    pub fn has_errors(&self) -> bool {
        self.check.has_errors()
    }
}

#[derive(Debug, Deserialize)]
struct RawWorkspaceConfig {
    module: Option<RawModuleConfig>,

    #[serde(default)]
    exports: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RawModuleConfig {
    name: Option<String>,
    target: Option<String>,
    entry: Option<String>,
}

pub(crate) fn check_workspace(root: impl AsRef<Path>) -> Result<WorkspaceCheckResult> {
    let root = root.as_ref();

    let root = if root.is_file() {
        root.parent().unwrap_or_else(|| Path::new(""))
    } else {
        root
    };

    let root = root.to_path_buf();
    let config_path = root.join("galfus.toml");

    let mut diagnostics = DiagnosticBag::new();

    if !config_path.exists() {
        diagnostics.push(Diagnostic::error_with_message(
            WorkspaceDiagnosticCode::MissingConfig,
            format!("missing `{}`", config_path.display()),
            workspace_span(),
        ));

        return Ok(empty_workspace_check_result(diagnostics));
    }

    let config_text = fs::read_to_string(config_path.as_path())?;
    let config = parse_workspace_config(root.as_path(), config_text.as_str(), &mut diagnostics);

    if diagnostics.has_errors() {
        return Ok(empty_workspace_check_result(diagnostics));
    }

    let Some(config) = config else {
        return Ok(empty_workspace_check_result(diagnostics));
    };

    let mut loader = ModuleLoader::default();

    if let Some(entry) = config.entry() {
        let entry = normalize_existing_path(entry)?;
        loader.load_module(entry)?;
    }

    for export in config.exports() {
        let path = normalize_existing_path(export.path())?;
        loader.load_module(path)?;
    }

    loader.validate_imports();

    let graph = WorkspaceGraph::from_workspace_config(&config, loader.modules.as_slice())?;

    let check = CheckResult {
        modules: loader.modules,
        diagnostics: loader.diagnostics,
    };

    Ok(WorkspaceCheckResult::new(check, graph))
}

fn parse_workspace_config(
    root: &Path,
    text: &str,
    diagnostics: &mut DiagnosticBag,
) -> Option<WorkspaceConfig> {
    let raw = match toml::from_str::<RawWorkspaceConfig>(text) {
        Ok(raw) => raw,
        Err(error) => {
            diagnostics.push(Diagnostic::error_with_message(
                WorkspaceDiagnosticCode::InvalidConfig,
                format!("invalid galfus.toml: {error}"),
                workspace_span(),
            ));
            return None;
        }
    };

    let Some(module) = raw.module else {
        diagnostics.push(Diagnostic::error(
            WorkspaceDiagnosticCode::MissingModuleTable,
            workspace_span(),
        ));
        return None;
    };

    let Some(name) = module.name else {
        diagnostics.push(Diagnostic::error(
            WorkspaceDiagnosticCode::MissingModuleName,
            workspace_span(),
        ));
        return None;
    };

    let Some(target_text) = module.target else {
        diagnostics.push(Diagnostic::error(
            WorkspaceDiagnosticCode::MissingModuleTarget,
            workspace_span(),
        ));
        return None;
    };

    let Some(target) = ModuleTarget::parse(target_text.as_str()) else {
        diagnostics.push(Diagnostic::error_with_message(
            WorkspaceDiagnosticCode::InvalidModuleTarget,
            format!("invalid module target `{target_text}`"),
            workspace_span(),
        ));
        return None;
    };

    let entry = module.entry.as_deref().map(|entry| root.join(entry));

    let exports = raw
        .exports
        .into_iter()
        .map(|(address, path)| WorkspaceExport {
            address,
            path: root.join(path),
        })
        .collect::<Vec<_>>();

    validate_workspace_surface(target, entry.as_deref(), exports.as_slice(), diagnostics);

    validate_workspace_paths(entry.as_deref(), exports.as_slice(), diagnostics);

    if diagnostics.has_errors() {
        return None;
    }

    Some(WorkspaceConfig {
        root: root.to_path_buf(),
        name,
        target,
        entry,
        exports,
    })
}

fn validate_workspace_surface(
    target: ModuleTarget,
    entry: Option<&Path>,
    exports: &[WorkspaceExport],
    diagnostics: &mut DiagnosticBag,
) {
    match target {
        ModuleTarget::App => {
            if entry.is_none() {
                diagnostics.push(Diagnostic::error(
                    WorkspaceDiagnosticCode::MissingAppEntry,
                    workspace_span(),
                ));
            }
        }
        ModuleTarget::Lib => {
            if entry.is_none() && exports.is_empty() {
                diagnostics.push(Diagnostic::error(
                    WorkspaceDiagnosticCode::MissingLibrarySurface,
                    workspace_span(),
                ));
            }
        }
    }
}

fn validate_workspace_paths(
    entry: Option<&Path>,
    exports: &[WorkspaceExport],
    diagnostics: &mut DiagnosticBag,
) {
    if let Some(entry) = entry {
        validate_source_target(
            entry,
            WorkspaceDiagnosticCode::EntryTargetMissing,
            "entry",
            diagnostics,
        );
    }

    for export in exports {
        validate_source_target(
            export.path(),
            WorkspaceDiagnosticCode::ExportTargetMissing,
            format!("export `{}`", export.address()).as_str(),
            diagnostics,
        );
    }
}

fn validate_source_target(
    path: &Path,
    missing_code: WorkspaceDiagnosticCode,
    label: &str,
    diagnostics: &mut DiagnosticBag,
) {
    if path.extension().and_then(|extension| extension.to_str()) != Some("gfs") {
        diagnostics.push(Diagnostic::error_with_message(
            WorkspaceDiagnosticCode::UnsupportedWorkspaceTarget,
            format!(
                "{label} must point to a .gfs source file: `{}`",
                path.display()
            ),
            workspace_span(),
        ));

        return;
    }

    if !path.exists() {
        diagnostics.push(Diagnostic::error_with_message(
            missing_code,
            format!("{label} target not found: `{}`", path.display()),
            workspace_span(),
        ));
    }
}

fn workspace_span() -> Span {
    Span::empty(WORKSPACE_SOURCE_ID, 0)
}

pub fn check_workspace_root(root: &str) -> Result<()> {
    let result = check_workspace(root)?;
    print_check_result(result.check_result());
    Ok(())
}

fn empty_workspace_check_result(diagnostics: DiagnosticBag) -> WorkspaceCheckResult {
    WorkspaceCheckResult::new(
        CheckResult {
            modules: Vec::new(),
            diagnostics,
        },
        WorkspaceGraph::default(),
    )
}
