use super::*;
use galfus_core::{NodeId, ScopeId, SymbolId};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ResolutionLayer {
    module_scope: ScopeId,
    scopes: Vec<Scope>,
    symbols: Vec<Symbol>,

    declarations: HashMap<NodeId, SymbolId>,
    references: HashMap<NodeId, SymbolId>,
    node_scopes: HashMap<NodeId, ScopeId>,
}

impl ResolutionLayer {
    pub fn new() -> Self {
        let module_scope = ScopeId::new(0);

        Self {
            module_scope,
            scopes: Vec::new(),
            symbols: Vec::new(),
            declarations: HashMap::new(),
            references: HashMap::new(),
            node_scopes: HashMap::new(),
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
}

impl Default for ResolutionLayer {
    fn default() -> Self {
        Self::new()
    }
}
