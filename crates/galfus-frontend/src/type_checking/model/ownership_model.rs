use galfus_core::{NodeId, SymbolId, TypeId};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnershipMetadata {
    pub(in crate::type_checking) anchors: Vec<AnchorMetadata>,
    pub(in crate::type_checking) edges: Vec<EdgeMetadata>,
    pub(in crate::type_checking) cycles: Vec<OwnershipCycleMetadata>,
    pub(in crate::type_checking) weak_observers: Vec<WeakObserverMetadata>,
    pub(in crate::type_checking) weak_fields: Vec<WeakFieldMetadata>,
    pub(in crate::type_checking) captures: Vec<CaptureMetadata>,
    pub(in crate::type_checking) temporaries: Vec<TemporaryMetadata>,
    pub(in crate::type_checking) release_eligibilities: Vec<ReleaseEligibilityMetadata>,
}

impl OwnershipMetadata {
    pub fn anchors(&self) -> &[AnchorMetadata] {
        &self.anchors
    }

    pub fn edges(&self) -> &[EdgeMetadata] {
        &self.edges
    }

    pub fn cycles(&self) -> &[OwnershipCycleMetadata] {
        &self.cycles
    }

    pub fn weak_observers(&self) -> &[WeakObserverMetadata] {
        &self.weak_observers
    }

    pub fn weak_fields(&self) -> &[WeakFieldMetadata] {
        &self.weak_fields
    }

    pub fn captures(&self) -> &[CaptureMetadata] {
        &self.captures
    }

    pub fn temporaries(&self) -> &[TemporaryMetadata] {
        &self.temporaries
    }

    pub fn release_eligibilities(&self) -> &[ReleaseEligibilityMetadata] {
        &self.release_eligibilities
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorKind {
    ModuleState,
    BlockLocal,
    FunctionParameter,
    FunctionAnchor,
    Closure,
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorMetadata {
    kind: AnchorKind,
    node: NodeId,
    symbol: Option<SymbolId>,
    ty: Option<TypeId>,
}

impl AnchorMetadata {
    pub fn new(
        kind: AnchorKind,
        node: NodeId,
        symbol: Option<SymbolId>,
        ty: Option<TypeId>,
    ) -> Self {
        Self {
            kind,
            node,
            symbol,
            ty,
        }
    }

    pub fn kind(&self) -> AnchorKind {
        self.kind
    }

    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn symbol(&self) -> Option<SymbolId> {
        self.symbol
    }

    pub fn ty(&self) -> Option<TypeId> {
        self.ty
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeMetadata {
    owner_symbol: SymbolId,
    field_symbol: SymbolId,
    declaration: NodeId,
    field_type: TypeId,
}

impl EdgeMetadata {
    pub fn new(
        owner_symbol: SymbolId,
        field_symbol: SymbolId,
        declaration: NodeId,
        field_type: TypeId,
    ) -> Self {
        Self {
            owner_symbol,
            field_symbol,
            declaration,
            field_type,
        }
    }

    pub fn owner_symbol(&self) -> SymbolId {
        self.owner_symbol
    }

    pub fn field_symbol(&self) -> SymbolId {
        self.field_symbol
    }

    pub fn declaration(&self) -> NodeId {
        self.declaration
    }

    pub fn field_type(&self) -> TypeId {
        self.field_type
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipCycleMetadata {
    structs: Vec<SymbolId>,
}

impl OwnershipCycleMetadata {
    pub fn new(structs: Vec<SymbolId>) -> Self {
        Self { structs }
    }

    pub fn structs(&self) -> &[SymbolId] {
        self.structs.as_slice()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeakObserverMetadata {
    owner_symbol: SymbolId,
    field_symbol: SymbolId,
    declaration: NodeId,
    field_type: TypeId,
}

impl WeakObserverMetadata {
    pub fn new(
        owner_symbol: SymbolId,
        field_symbol: SymbolId,
        declaration: NodeId,
        field_type: TypeId,
    ) -> Self {
        Self {
            owner_symbol,
            field_symbol,
            declaration,
            field_type,
        }
    }

    pub fn owner_symbol(&self) -> SymbolId {
        self.owner_symbol
    }

    pub fn field_symbol(&self) -> SymbolId {
        self.field_symbol
    }

    pub fn declaration(&self) -> NodeId {
        self.declaration
    }

    pub fn field_type(&self) -> TypeId {
        self.field_type
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeakFieldMetadata {
    struct_symbol: SymbolId,
    field_symbol: SymbolId,
    declaration: NodeId,
    field_type: TypeId,
}

impl WeakFieldMetadata {
    pub fn new(
        struct_symbol: SymbolId,
        field_symbol: SymbolId,
        declaration: NodeId,
        field_type: TypeId,
    ) -> Self {
        Self {
            struct_symbol,
            field_symbol,
            declaration,
            field_type,
        }
    }

    pub fn struct_symbol(&self) -> SymbolId {
        self.struct_symbol
    }

    pub fn field_symbol(&self) -> SymbolId {
        self.field_symbol
    }

    pub fn declaration(&self) -> NodeId {
        self.declaration
    }

    pub fn field_type(&self) -> TypeId {
        self.field_type
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureMetadata {
    closure: NodeId,
    reference: NodeId,
    symbol: SymbolId,
    ty: TypeId,
}

impl CaptureMetadata {
    pub fn new(closure: NodeId, reference: NodeId, symbol: SymbolId, ty: TypeId) -> Self {
        Self {
            closure,
            reference,
            symbol,
            ty,
        }
    }

    pub fn closure(&self) -> NodeId {
        self.closure
    }

    pub fn reference(&self) -> NodeId {
        self.reference
    }

    pub fn symbol(&self) -> SymbolId {
        self.symbol
    }

    pub fn ty(&self) -> TypeId {
        self.ty
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporaryMetadata {
    expression: NodeId,
    ty: TypeId,
}

impl TemporaryMetadata {
    pub fn new(expression: NodeId, ty: TypeId) -> Self {
        Self { expression, ty }
    }

    pub fn expression(&self) -> NodeId {
        self.expression
    }

    pub fn ty(&self) -> TypeId {
        self.ty
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReleaseEligibilityKind {
    Anchor,
    Capture,
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseEligibilityMetadata {
    kind: ReleaseEligibilityKind,
    node: NodeId,
    symbol: Option<SymbolId>,
    ty: TypeId,
}

impl ReleaseEligibilityMetadata {
    pub fn new(
        kind: ReleaseEligibilityKind,
        node: NodeId,
        symbol: Option<SymbolId>,
        ty: TypeId,
    ) -> Self {
        Self {
            kind,
            node,
            symbol,
            ty,
        }
    }

    pub fn kind(&self) -> ReleaseEligibilityKind {
        self.kind
    }

    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn symbol(&self) -> Option<SymbolId> {
        self.symbol
    }

    pub fn ty(&self) -> TypeId {
        self.ty
    }
}
