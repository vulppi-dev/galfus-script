use galfus_core::{NodeId, ScopeId, SymbolId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    TypeAlias,
    Struct,
    Enum,
    Choice,
    Constraint,

    Var,
    Const,

    Parameter,
    RestParameter,
    GenericParameter,

    StructField,
    EnumVariant,
    ChoiceVariant,

    ImportNamespace,
    ImportBinding,

    BuiltinType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    id: SymbolId,
    kind: SymbolKind,
    name: String,
    declaration: NodeId,
    scope: ScopeId,
}

impl Symbol {
    pub fn new(
        id: SymbolId,
        kind: SymbolKind,
        name: String,
        declaration: NodeId,
        scope: ScopeId,
    ) -> Self {
        Self {
            id,
            kind,
            name,
            declaration,
            scope,
        }
    }

    pub fn id(&self) -> SymbolId {
        self.id
    }

    pub fn kind(&self) -> SymbolKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn declaration(&self) -> NodeId {
        self.declaration
    }

    pub fn scope(&self) -> ScopeId {
        self.scope
    }
}
