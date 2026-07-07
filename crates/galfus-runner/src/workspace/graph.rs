use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use galfus_core::NodeId;
use galfus_frontend::{ImportKind, ImportRecord, SyntaxNodeKind};

use crate::CheckedModule;

use super::config::WorkspaceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceModuleId(usize);

impl WorkspaceModuleId {
    pub fn new(raw: usize) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceRootKind {
    Entry,
    Export { address: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRoot {
    kind: WorkspaceRootKind,
    module_id: WorkspaceModuleId,
    path: PathBuf,
}

impl WorkspaceRoot {
    pub fn kind(&self) -> &WorkspaceRootKind {
        &self.kind
    }

    pub fn module_id(&self) -> WorkspaceModuleId {
        self.module_id
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceModule {
    id: WorkspaceModuleId,
    path: PathBuf,
}

impl WorkspaceModule {
    pub fn id(&self) -> WorkspaceModuleId {
        self.id
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceImportEdge {
    from: WorkspaceModuleId,
    kind: ImportKind,
    source: String,
    source_node: NodeId,
    local_name: String,
    imported_name: Option<String>,
    target_path: PathBuf,
    to: Option<WorkspaceModuleId>,
    export_name: Option<String>,
    referenced_exports: Vec<String>,
}

impl WorkspaceImportEdge {
    pub fn from(&self) -> WorkspaceModuleId {
        self.from
    }

    pub fn kind(&self) -> ImportKind {
        self.kind
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    pub fn source_node(&self) -> NodeId {
        self.source_node
    }

    pub fn local_name(&self) -> &str {
        self.local_name.as_str()
    }

    pub fn imported_name(&self) -> Option<&str> {
        self.imported_name.as_deref()
    }

    pub fn target_path(&self) -> &Path {
        self.target_path.as_path()
    }

    pub fn to(&self) -> Option<WorkspaceModuleId> {
        self.to
    }

    pub fn is_resolved(&self) -> bool {
        self.to.is_some()
    }

    pub fn export_name(&self) -> Option<&str> {
        self.export_name.as_deref()
    }

    pub fn is_export_resolved(&self) -> bool {
        self.export_name.is_some()
    }

    pub fn referenced_exports(&self) -> &[String] {
        self.referenced_exports.as_slice()
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceGraph {
    roots: Vec<WorkspaceRoot>,
    modules: Vec<WorkspaceModule>,
    module_by_path: HashMap<PathBuf, WorkspaceModuleId>,
    import_edges: Vec<WorkspaceImportEdge>,
}

impl WorkspaceGraph {
    pub fn from_workspace_config(
        config: &WorkspaceConfig,
        checked_modules: &[CheckedModule],
    ) -> Result<Self> {
        let mut graph = Self::default();

        for (index, module) in checked_modules.iter().enumerate() {
            let id = WorkspaceModuleId::new(index);
            let path = module.path().to_path_buf();

            graph.modules.push(WorkspaceModule {
                id,
                path: path.clone(),
            });

            graph.module_by_path.insert(path, id);
        }

        graph.add_roots(config)?;
        graph.add_import_edges(checked_modules)?;

        Ok(graph)
    }

    pub fn for_single_file(entry_path: &Path, checked_modules: &[CheckedModule]) -> Result<Self> {
        let mut graph = Self::default();

        for (index, module) in checked_modules.iter().enumerate() {
            let id = WorkspaceModuleId::new(index);
            let path = module.path().to_path_buf();

            graph.modules.push(WorkspaceModule {
                id,
                path: path.clone(),
            });

            graph.module_by_path.insert(path, id);
        }

        let entry_canonical = crate::normalize_existing_path(entry_path)?;
        if let Some(entry_id) = graph.module_by_path.get(&entry_canonical).copied() {
            graph.roots.push(WorkspaceRoot {
                kind: WorkspaceRootKind::Entry,
                module_id: entry_id,
                path: entry_canonical,
            });
        }

        graph.add_import_edges(checked_modules)?;

        Ok(graph)
    }

    pub fn roots(&self) -> &[WorkspaceRoot] {
        self.roots.as_slice()
    }

    pub fn modules(&self) -> &[WorkspaceModule] {
        self.modules.as_slice()
    }

    pub fn import_edges(&self) -> &[WorkspaceImportEdge] {
        self.import_edges.as_slice()
    }

    pub fn module_by_path(&self, path: &Path) -> Option<WorkspaceModuleId> {
        self.module_by_path.get(path).copied()
    }

    fn add_roots(&mut self, config: &WorkspaceConfig) -> Result<()> {
        if let Some(entry) = config.entry() {
            let path = canonical_path(entry)?;

            if let Some(module_id) = self.module_by_path(path.as_path()) {
                self.roots.push(WorkspaceRoot {
                    kind: WorkspaceRootKind::Entry,
                    module_id,
                    path,
                });
            }
        }

        for export in config.exports() {
            let path = canonical_path(export.path())?;

            if let Some(module_id) = self.module_by_path(path.as_path()) {
                self.roots.push(WorkspaceRoot {
                    kind: WorkspaceRootKind::Export {
                        address: export.address().to_string(),
                    },
                    module_id,
                    path,
                });
            }
        }

        Ok(())
    }

    fn add_import_edges(&mut self, checked_modules: &[CheckedModule]) -> Result<()> {
        for module in checked_modules {
            let Some(from) = self.module_by_path(module.path()) else {
                continue;
            };

            let Some(resolution) = module.graph().resolution() else {
                continue;
            };

            for import in resolution.imports() {
                let source = import.source();

                if !is_relative_import(source) {
                    continue;
                }

                let target_path = resolve_relative_import(module.path(), source);

                let target_path = if target_path.exists() {
                    canonical_path(target_path.as_path())?
                } else {
                    target_path
                };

                let to = self.module_by_path(target_path.as_path());

                let export_name = self.resolve_import_export(import, to, checked_modules);
                let referenced_exports =
                    self.resolve_namespace_referenced_exports(module, import, to, checked_modules);

                self.import_edges.push(WorkspaceImportEdge {
                    from,
                    kind: import.kind(),
                    source: source.to_string(),
                    source_node: import.source_node(),
                    local_name: import.local_name().to_string(),
                    imported_name: import.imported_name().map(str::to_string),
                    target_path,
                    to,
                    export_name,
                    referenced_exports,
                });
            }
        }

        Ok(())
    }

    fn resolve_import_export(
        &self,
        import: &ImportRecord,
        to: Option<WorkspaceModuleId>,
        checked_modules: &[CheckedModule],
    ) -> Option<String> {
        if import.kind() != ImportKind::Named {
            return None;
        }

        let target_module = checked_modules.get(to?.raw())?;
        let target_resolution = target_module.graph().resolution()?;
        let imported_name = import.imported_name()?;

        target_resolution
            .export_by_name(imported_name)
            .map(|_| imported_name.to_string())
    }

    fn resolve_namespace_referenced_exports(
        &self,
        module: &CheckedModule,
        import: &ImportRecord,
        to: Option<WorkspaceModuleId>,
        checked_modules: &[CheckedModule],
    ) -> Vec<String> {
        if import.kind() != ImportKind::Namespace {
            return Vec::new();
        }

        let Some(to) = to else {
            return Vec::new();
        };

        let Some(target_module) = checked_modules.get(to.raw()) else {
            return Vec::new();
        };

        let Some(target_resolution) = target_module.graph().resolution() else {
            return Vec::new();
        };

        let mut references = self.namespace_reference_names(module, import.local_name());

        references.retain(|name| target_resolution.export_by_name(name.as_str()).is_some());
        references.sort();
        references.dedup();

        references
    }

    fn namespace_reference_names(&self, module: &CheckedModule, namespace: &str) -> Vec<String> {
        let Some(root) = module.graph().syntax().root() else {
            return Vec::new();
        };

        let mut references = Vec::new();
        self.collect_namespace_reference_names(module, root, namespace, &mut references);
        references
    }

    fn collect_namespace_reference_names(
        &self,
        module: &CheckedModule,
        node: NodeId,
        namespace: &str,
        references: &mut Vec<String>,
    ) {
        let syntax = module.graph().syntax();

        let Some(syntax_node) = syntax.node(node) else {
            return;
        };

        if matches!(
            syntax_node.kind(),
            SyntaxNodeKind::PathExpression | SyntaxNodeKind::Path
        ) {
            if let Some(reference) = self.namespace_reference_name(module, node, namespace) {
                references.push(reference);
            }

            return;
        }

        for child in syntax_node.children() {
            self.collect_namespace_reference_names(module, *child, namespace, references);
        }
    }

    fn namespace_reference_name(
        &self,
        module: &CheckedModule,
        node: NodeId,
        namespace: &str,
    ) -> Option<String> {
        let segments = self.path_segments(module, node);
        let root = segments.first()?;

        if root != namespace || segments.len() < 2 {
            return None;
        }

        Some(segments[1..].join("::"))
    }

    fn path_segments(&self, module: &CheckedModule, node: NodeId) -> Vec<String> {
        let syntax = module.graph().syntax();
        let Some(syntax_node) = syntax.node(node) else {
            return Vec::new();
        };

        match syntax_node.kind() {
            SyntaxNodeKind::NameExpression => syntax
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)
                .map(|identifier| self.node_text(module, identifier))
                .into_iter()
                .collect(),

            SyntaxNodeKind::PathExpression => {
                let Some(target) = syntax.child(node, 0) else {
                    return Vec::new();
                };

                let Some(member) = syntax.child(node, 1) else {
                    return Vec::new();
                };

                let mut segments = self.path_segments(module, target);
                segments.push(self.node_text(module, member));
                segments
            }

            SyntaxNodeKind::Path => syntax_node
                .children()
                .iter()
                .filter_map(|child| {
                    let child_node = syntax.node(*child)?;

                    if child_node.kind() != SyntaxNodeKind::Identifier {
                        return None;
                    }

                    Some(self.node_text(module, *child))
                })
                .collect(),

            _ => Vec::new(),
        }
    }

    fn node_text(&self, module: &CheckedModule, node: NodeId) -> String {
        let Some(syntax_node) = module.graph().syntax().node(node) else {
            return String::new();
        };

        module
            .source()
            .slice(syntax_node.span())
            .unwrap_or("")
            .to_string()
    }
}

fn canonical_path(path: &Path) -> Result<PathBuf> {
    Ok(path.canonicalize()?)
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
