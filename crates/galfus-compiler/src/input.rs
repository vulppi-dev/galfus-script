use galfus_core::{ModulePath, SourceFile};
use galfus_frontend::{ModuleGraph, TypeCheckResult};

/// A single verified module that can be fed into the compiler.
///
/// This type serves as the boundary between the frontend (checking) phase and
/// the compilation phase. It is intentionally independent of filesystem
/// concerns: the `path` is used only as a stable identifier for cross-module
/// linking, not for I/O.
pub struct CompiledModule {
    /// Logical path used as a stable module identifier for cross-module linking.
    pub(crate) path: ModulePath,
    pub(crate) source: SourceFile,
    pub(crate) graph: ModuleGraph,
    pub(crate) type_result: Option<TypeCheckResult>,
}

impl CompiledModule {
    pub fn new(
        path: ModulePath,
        source: SourceFile,
        graph: ModuleGraph,
        type_result: Option<TypeCheckResult>,
    ) -> Self {
        Self {
            path,
            source,
            graph,
            type_result,
        }
    }

    pub fn path(&self) -> &ModulePath {
        &self.path
    }

    pub fn source(&self) -> &SourceFile {
        &self.source
    }

    pub fn graph(&self) -> &ModuleGraph {
        &self.graph
    }

    pub fn type_result(&self) -> Option<&TypeCheckResult> {
        self.type_result.as_ref()
    }

    pub fn type_result_mut(&mut self) -> Option<&mut TypeCheckResult> {
        self.type_result.as_mut()
    }
}

/// The input to the compiler: a set of verified modules with a declared entry point.
pub struct CompilerInput<'a> {
    /// All modules to be compiled, in dependency order (dependencies before dependents).
    pub modules: &'a mut [CompiledModule],
    /// Index within `modules` of the workspace entry point.
    pub entry_index: usize,
    /// The name of the module image to produce (e.g. the workspace name).
    pub image_name: String,
}
