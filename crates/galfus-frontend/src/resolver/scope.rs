use galfus_core::{NodeId, ScopeId, SymbolId};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    Module,
    Function,
    Block,
    TypeAlias,
    Struct,
    Enum,
    Choice,
    Constraint,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    id: ScopeId,
    kind: ScopeKind,
    parent: Option<ScopeId>,
    owner: Option<NodeId>,
    symbols: HashMap<String, SymbolId>,
}

impl Scope {
    pub fn new(
        id: ScopeId,
        kind: ScopeKind,
        parent: Option<ScopeId>,
        owner: Option<NodeId>,
    ) -> Self {
        Self {
            id,
            kind,
            parent,
            owner,
            symbols: HashMap::new(),
        }
    }

    pub fn id(&self) -> ScopeId {
        self.id
    }

    pub fn kind(&self) -> ScopeKind {
        self.kind
    }

    pub fn parent(&self) -> Option<ScopeId> {
        self.parent
    }

    pub fn owner(&self) -> Option<NodeId> {
        self.owner
    }

    pub fn symbols(&self) -> &HashMap<String, SymbolId> {
        &self.symbols
    }

    pub fn symbol(&self, name: &str) -> Option<SymbolId> {
        self.symbols.get(name).copied()
    }

    pub(crate) fn insert_symbol(&mut self, name: String, symbol: SymbolId) -> Option<SymbolId> {
        self.symbols.insert(name, symbol)
    }
}
