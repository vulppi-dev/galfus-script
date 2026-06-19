use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use galfus_core::NodeId;

use crate::*;

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
    source: String,
    source_node: NodeId,
    target_path: PathBuf,
    to: Option<WorkspaceModuleId>,
}

impl WorkspaceImportEdge {
    pub fn from(&self) -> WorkspaceModuleId {
        self.from
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    pub fn source_node(&self) -> NodeId {
        self.source_node
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

                self.import_edges.push(WorkspaceImportEdge {
                    from,
                    source: source.to_string(),
                    source_node: import.source_node(),
                    target_path,
                    to,
                });
            }
        }

        Ok(())
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
