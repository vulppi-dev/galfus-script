use anyhow::Result;
use galfus_core::DiagnosticBag;
use std::{fs, path::Path};

use crate::{CheckResult, CheckedModule, ModuleLoader, WorkspaceDiagnosticCode};

use super::config::{parse_workspace_config, workspace_span};
use super::graph::WorkspaceGraph;

#[derive(Debug, Clone)]
pub struct WorkspaceCheckResult {
    check: CheckResult,
    graph: WorkspaceGraph,
    run_entry: String,
    run_args: Vec<String>,
}

impl WorkspaceCheckResult {
    pub fn new(check: CheckResult, graph: WorkspaceGraph) -> Self {
        Self {
            check,
            graph,
            run_entry: "main".to_string(),
            run_args: Vec::new(),
        }
    }

    pub fn with_run_entry(mut self, run_entry: impl Into<String>) -> Self {
        self.run_entry = run_entry.into();
        self
    }

    pub fn with_run_args(mut self, run_args: Vec<String>) -> Self {
        self.run_args = run_args;
        self
    }

    pub fn check_result(&self) -> &CheckResult {
        &self.check
    }

    pub fn graph(&self) -> &WorkspaceGraph {
        &self.graph
    }

    pub fn run_entry(&self) -> &str {
        self.run_entry.as_str()
    }

    pub fn run_args(&self) -> &[String] {
        self.run_args.as_slice()
    }

    pub fn modules(&self) -> &[CheckedModule] {
        self.check.modules()
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        self.check.diagnostics()
    }

    pub fn has_errors(&self) -> bool {
        self.check.has_errors()
    }
}

pub fn check_workspace(root: impl AsRef<Path>) -> Result<WorkspaceCheckResult> {
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
        diagnostics.push(galfus_core::Diagnostic::error_with_message(
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
        let entry = crate::normalize_existing_path(entry)?;
        loader.load_module(entry)?;
    }

    for export in config.exports() {
        let path = crate::normalize_existing_path(export.path())?;
        loader.load_module(path)?;
    }

    loader.validate_imports();
    loader.type_check_modules();

    let graph = WorkspaceGraph::from_workspace_config(&config, loader.modules.as_slice())?;

    let check = CheckResult {
        modules: loader.modules,
        diagnostics: loader.diagnostics,
    };

    Ok(WorkspaceCheckResult::new(check, graph)
        .with_run_entry(config.run_entry())
        .with_run_args(config.run_args.clone()))
}

pub fn check_workspace_root(root: &str) -> Result<()> {
    let result = check_workspace(root)?;
    crate::print_check_result(result.check_result());
    Ok(())
}

pub(super) fn empty_workspace_check_result(diagnostics: DiagnosticBag) -> WorkspaceCheckResult {
    WorkspaceCheckResult::new(
        CheckResult {
            modules: Vec::new(),
            diagnostics,
        },
        WorkspaceGraph::default(),
    )
}
