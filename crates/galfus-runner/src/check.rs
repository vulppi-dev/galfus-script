use crate::*;
use anyhow::Result;
use galfus_core::{Diagnostic, DiagnosticBag, NodeId, SourceFile, SourceId};
use galfus_frontend::{ImportKind, ModuleGraph, SyntaxNodeKind, parse, resolve};
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

#[derive(Debug, Clone)]
struct ImportCheckRecord {
    kind: ImportKind,
    source: String,
    local_name: String,
    imported_name: Option<String>,
    declaration: NodeId,
}

#[derive(Debug, Clone)]
struct PathSegmentRecord {
    name: String,
    node: NodeId,
}

#[derive(Debug, Clone)]
struct PathCheckRecord {
    segments: Vec<PathSegmentRecord>,
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

            if path.extension().and_then(|extension| extension.to_str()) != Some("gfs") {
                let span = self.modules[module_index]
                    .graph()
                    .syntax()
                    .node(source_node)
                    .map(|node| node.span())
                    .unwrap_or_else(|| self.modules[module_index].source().span());

                self.diagnostics.push(Diagnostic::error_with_message(
                    CheckDiagnosticCode::UnsupportedImportTarget,
                    format!("import `{source}` must resolve to a .gfs source file"),
                    span,
                ));

                continue;
            }

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

        self.validate_namespace_import_references();
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
                local_name: import.local_name().to_string(),
                imported_name: import.imported_name().map(str::to_string),
                declaration: import.declaration(),
            })
            .collect()
    }

    fn path_records(&self, module_index: usize) -> Vec<PathCheckRecord> {
        let mut records = Vec::new();

        let Some(root) = self.modules[module_index].graph().syntax().root() else {
            return records;
        };

        self.collect_path_records(module_index, root, &mut records);

        records
    }

    fn collect_path_records(
        &self,
        module_index: usize,
        node: NodeId,
        records: &mut Vec<PathCheckRecord>,
    ) {
        let syntax = self.modules[module_index].graph().syntax();

        let Some(syntax_node) = syntax.node(node) else {
            return;
        };

        match syntax_node.kind() {
            SyntaxNodeKind::PathExpression => {
                let segments = self.path_expression_segments(module_index, node);

                if segments.len() >= 2 {
                    records.push(PathCheckRecord { segments });
                }

                return;
            }

            SyntaxNodeKind::Path => {
                let segments = self.type_path_segments(module_index, node);

                if segments.len() >= 2 {
                    records.push(PathCheckRecord { segments });
                }

                return;
            }

            _ => {}
        }

        for child in syntax_node.children() {
            self.collect_path_records(module_index, *child, records);
        }
    }

    fn path_expression_segments(
        &self,
        module_index: usize,
        expression: NodeId,
    ) -> Vec<PathSegmentRecord> {
        let syntax = self.modules[module_index].graph().syntax();

        let Some(expression_node) = syntax.node(expression) else {
            return Vec::new();
        };

        match expression_node.kind() {
            SyntaxNodeKind::NameExpression => {
                let Some(identifier) =
                    syntax.first_child_of_kind(expression, SyntaxNodeKind::Identifier)
                else {
                    return Vec::new();
                };

                vec![PathSegmentRecord {
                    name: self.node_text(module_index, identifier),
                    node: identifier,
                }]
            }

            SyntaxNodeKind::PathExpression => {
                let Some(target) = syntax.child(expression, 0) else {
                    return Vec::new();
                };

                let Some(member) = syntax.child(expression, 1) else {
                    return Vec::new();
                };

                let mut segments = self.path_expression_segments(module_index, target);

                segments.push(PathSegmentRecord {
                    name: self.node_text(module_index, member),
                    node: member,
                });

                segments
            }

            _ => Vec::new(),
        }
    }

    fn type_path_segments(&self, module_index: usize, path: NodeId) -> Vec<PathSegmentRecord> {
        let syntax = self.modules[module_index].graph().syntax();

        let Some(path_node) = syntax.node(path) else {
            return Vec::new();
        };

        path_node
            .children()
            .iter()
            .filter_map(|child| {
                let child_node = syntax.node(*child)?;

                if child_node.kind() != SyntaxNodeKind::Identifier {
                    return None;
                }

                Some(PathSegmentRecord {
                    name: self.node_text(module_index, *child),
                    node: *child,
                })
            })
            .collect()
    }

    fn node_text(&self, module_index: usize, node: NodeId) -> String {
        let Some(syntax_node) = self.modules[module_index].graph().syntax().node(node) else {
            return String::new();
        };

        self.modules[module_index]
            .source()
            .slice(syntax_node.span())
            .unwrap_or("")
            .to_string()
    }

    fn validate_namespace_import_references(&mut self) {
        for module_index in 0..self.modules.len() {
            let imports = self.module_imports(module_index);

            let namespace_imports = imports
                .iter()
                .filter(|import| {
                    import.kind == ImportKind::Namespace
                        && is_relative_import(import.source.as_str())
                })
                .collect::<Vec<_>>();

            if namespace_imports.is_empty() {
                continue;
            }

            let paths = self.path_records(module_index);

            for path in paths {
                let Some(root_segment) = path.segments.first() else {
                    continue;
                };

                let Some(import) = namespace_imports
                    .iter()
                    .find(|import| import.local_name == root_segment.name)
                else {
                    continue;
                };

                let target_path = resolve_relative_import(
                    self.modules[module_index].path(),
                    import.source.as_str(),
                );

                let Ok(target_path) = normalize_existing_path(target_path.as_path()) else {
                    continue;
                };

                let Some(target_index) = self.module_by_path.get(target_path.as_path()).copied()
                else {
                    continue;
                };

                let Some(target_resolution) = self.modules[target_index].graph().resolution()
                else {
                    continue;
                };

                let exported_name = path.segments[1..]
                    .iter()
                    .map(|segment| segment.name.as_str())
                    .collect::<Vec<_>>()
                    .join("::");

                if target_resolution
                    .export_by_name(exported_name.as_str())
                    .is_some()
                {
                    continue;
                }

                let span = self.modules[module_index]
                    .graph()
                    .syntax()
                    .node(path.segments[1].node)
                    .map(|node| node.span())
                    .unwrap_or_else(|| self.modules[module_index].source().span());

                self.diagnostics.push(Diagnostic::error_with_message(
                    CheckDiagnosticCode::MissingExport,
                    format!(
                        "module `{}` does not export `{}`",
                        import.source, exported_name
                    ),
                    span,
                ));
            }
        }
    }
}

pub(crate) fn check_path(path: impl AsRef<Path>) -> Result<CheckResult> {
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
