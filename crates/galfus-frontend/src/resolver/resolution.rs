use super::*;
use galfus_core::{ExportId, ImportId, NodeId, ScopeId, SymbolId};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ResolutionLayer {
    module_scope: ScopeId,
    scopes: Vec<Scope>,
    symbols: Vec<Symbol>,
    imports: Vec<ImportRecord>,
    exports: Vec<ExportRecord>,

    declarations: HashMap<NodeId, SymbolId>,
    references: HashMap<NodeId, SymbolId>,
    node_scopes: HashMap<NodeId, ScopeId>,
    symbol_imports: HashMap<SymbolId, ImportId>,
    exports_by_name: HashMap<String, ExportId>,
    symbol_exports: HashMap<SymbolId, ExportId>,
}

impl ResolutionLayer {
    pub fn new() -> Self {
        let module_scope = ScopeId::new(0);

        Self {
            module_scope,
            scopes: Vec::new(),
            symbols: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),

            declarations: HashMap::new(),
            references: HashMap::new(),
            node_scopes: HashMap::new(),
            symbol_imports: HashMap::new(),
            exports_by_name: HashMap::new(),
            symbol_exports: HashMap::new(),
        }
    }

    pub fn module_scope(&self) -> ScopeId {
        self.module_scope
    }

    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(id.raw() as usize)
    }

    pub fn scope_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        self.scopes.get_mut(id.raw() as usize)
    }

    pub fn symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.raw() as usize)
    }

    pub fn declaration_symbol(&self, node: NodeId) -> Option<SymbolId> {
        self.declarations.get(&node).copied()
    }

    pub fn reference_symbol(&self, node: NodeId) -> Option<SymbolId> {
        self.references.get(&node).copied()
    }

    pub fn node_scope(&self, node: NodeId) -> Option<ScopeId> {
        self.node_scopes.get(&node).copied()
    }

    pub fn imports(&self) -> &[ImportRecord] {
        self.imports.as_slice()
    }

    pub fn import(&self, id: ImportId) -> Option<&ImportRecord> {
        self.imports.get(id.raw() as usize)
    }

    pub fn import_for_symbol(&self, symbol: SymbolId) -> Option<ImportId> {
        self.symbol_imports.get(&symbol).copied()
    }

    pub fn exports(&self) -> &[ExportRecord] {
        self.exports.as_slice()
    }

    pub fn export_record(&self, id: ExportId) -> Option<&ExportRecord> {
        self.exports.get(id.raw() as usize)
    }

    pub fn export_by_name(&self, name: &str) -> Option<ExportId> {
        self.exports_by_name.get(name).copied()
    }

    pub fn export_for_symbol(&self, symbol: SymbolId) -> Option<ExportId> {
        self.symbol_exports.get(&symbol).copied()
    }

    pub(crate) fn add_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<ScopeId>,
        owner: Option<NodeId>,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);

        if self.scopes.is_empty() {
            self.module_scope = id;
        }

        self.scopes.push(Scope::new(id, kind, parent, owner));

        if let Some(owner) = owner {
            self.bind_scope(owner, id);
        }

        id
    }

    pub(crate) fn add_symbol(
        &mut self,
        kind: SymbolKind,
        name: String,
        declaration: NodeId,
        scope: ScopeId,
    ) -> SymbolId {
        let id = SymbolId::new(self.symbols.len() as u32);

        self.symbols
            .push(Symbol::new(id, kind, name, declaration, scope));

        self.declarations.insert(declaration, id);

        id
    }

    pub(crate) fn bind_reference(&mut self, node: NodeId, symbol: SymbolId) {
        self.references.insert(node, symbol);
    }

    pub(crate) fn bind_scope(&mut self, node: NodeId, scope: ScopeId) {
        self.node_scopes.insert(node, scope);
    }

    pub(crate) fn add_import(
        &mut self,
        kind: ImportKind,
        source: String,
        source_node: NodeId,
        import_node: NodeId,
        declaration: NodeId,
        local_name: String,
        imported_name: Option<String>,
        local_symbol: SymbolId,
    ) -> ImportId {
        let id = ImportId::new(self.imports.len() as u32);

        self.imports.push(ImportRecord::new(
            id,
            kind,
            source,
            source_node,
            import_node,
            declaration,
            local_name,
            imported_name,
            local_symbol,
        ));

        self.symbol_imports.insert(local_symbol, id);

        id
    }

    pub(crate) fn add_export(
        &mut self,
        name: String,
        kind: SymbolKind,
        export_node: NodeId,
        item_node: NodeId,
        declaration: NodeId,
        symbol: SymbolId,
    ) -> ExportId {
        if let Some(existing) = self.symbol_exports.get(&symbol).copied() {
            return existing;
        }

        if let Some(existing) = self.exports_by_name.get(name.as_str()).copied() {
            return existing;
        }

        let id = ExportId::new(self.exports.len() as u32);

        self.exports.push(ExportRecord::new(
            id,
            name.clone(),
            kind,
            export_node,
            item_node,
            declaration,
            symbol,
        ));

        self.exports_by_name.insert(name, id);
        self.symbol_exports.insert(symbol, id);

        id
    }
}

impl Default for ResolutionLayer {
    fn default() -> Self {
        Self::new()
    }
}
