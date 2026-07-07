use galfus_core::{Diagnostic, DiagnosticBag, Span};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::WorkspaceDiagnosticCode;
use galfus_core::SourceId;

pub(crate) const WORKSPACE_SOURCE_ID: SourceId = SourceId::new(u32::MAX);

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
    pub(super) entry: Option<PathBuf>,
    pub(super) run_entry: String,
    pub(super) run_args: Vec<String>,
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

    pub fn run_entry(&self) -> &str {
        self.run_entry.as_str()
    }

    pub fn run_args(&self) -> &[String] {
        self.run_args.as_slice()
    }

    pub fn exports(&self) -> &[WorkspaceExport] {
        self.exports.as_slice()
    }
}

#[derive(Debug, Deserialize)]
struct RawWorkspaceConfig {
    module: Option<RawModuleConfig>,
    run: Option<RawRunConfig>,

    #[serde(default)]
    exports: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RawModuleConfig {
    name: Option<String>,
    target: Option<String>,
    entry: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawRunConfig {
    entry: Option<String>,
    args: Option<Vec<String>>,
}

pub(super) fn parse_workspace_config(
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

    let run_entry = raw
        .run
        .as_ref()
        .and_then(|run| run.entry.clone())
        .unwrap_or_else(|| "main".to_string());

    let run_args = raw
        .run
        .as_ref()
        .and_then(|run| run.args.clone())
        .unwrap_or_default();

    if run_entry.contains('.') || run_entry.contains('/') || run_entry.contains('\\') {
        diagnostics.push(Diagnostic::error_with_message(
            WorkspaceDiagnosticCode::InvalidConfig,
            "`run.entry` must be the exported function name from the entry module".to_string(),
            workspace_span(),
        ));
        return None;
    }

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
        run_entry,
        run_args,
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

pub(super) fn workspace_span() -> Span {
    Span::empty(WORKSPACE_SOURCE_ID, 0)
}
