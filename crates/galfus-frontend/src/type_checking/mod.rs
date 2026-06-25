#[cfg(test)]
mod tests;

mod access;
mod arrow_functions;
mod assignability;
mod assignments;
mod builtin_constraints;
mod calls;
mod constraints;
mod control_flow;
mod declarations;
mod decorators;
mod diagnostics;
mod enums;
mod expressions;
mod function_stamps;
mod generic_expressions;
mod inferred_structs;
mod initializers;
mod instanceof;
mod literals;
mod matches;
mod operators;
mod ownership;
mod ranges;
mod returns;
mod semantics;
mod structs;
mod support;
mod variants;

use std::collections::HashMap;

use galfus_core::{DiagnosticBag, NodeId, SourceFile, SymbolId, TypeId};

use crate::{ArraySize, FunctionParameterType, ModuleGraph, PrimitiveType, TypeLayer, lower_types};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedFunctionParameterType {
    ty: ImportedType,
    is_rest: bool,
    has_default: bool,
}

impl ImportedFunctionParameterType {
    pub fn new(ty: ImportedType) -> Self {
        Self {
            ty,
            is_rest: false,
            has_default: false,
        }
    }

    pub fn rest(ty: ImportedType) -> Self {
        Self {
            ty,
            is_rest: true,
            has_default: false,
        }
    }

    pub fn with_default(ty: ImportedType) -> Self {
        Self {
            ty,
            is_rest: false,
            has_default: true,
        }
    }

    pub fn ty(&self) -> &ImportedType {
        &self.ty
    }

    pub fn is_rest(&self) -> bool {
        self.is_rest
    }

    pub fn has_default(&self) -> bool {
        self.has_default
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportedType {
    Primitive(PrimitiveType),
    NamedLocal {
        symbol: SymbolId,
    },
    Array {
        element: Box<ImportedType>,
    },
    FixedArray {
        element: Box<ImportedType>,
        size: ArraySize,
    },
    Range {
        element: Box<ImportedType>,
    },
    Tuple {
        elements: Vec<ImportedType>,
    },
    Union {
        members: Vec<ImportedType>,
    },
    Function {
        parameters: Vec<ImportedFunctionParameterType>,
        return_type: Box<ImportedType>,
    },
}

#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    layer: TypeLayer,
    diagnostics: DiagnosticBag,
    ownership_metadata: OwnershipMetadata,
}

impl TypeCheckResult {
    pub fn new(layer: TypeLayer, diagnostics: DiagnosticBag) -> Self {
        Self::with_ownership_metadata(layer, diagnostics, OwnershipMetadata::default())
    }

    fn with_ownership_metadata(
        layer: TypeLayer,
        diagnostics: DiagnosticBag,
        ownership_metadata: OwnershipMetadata,
    ) -> Self {
        Self {
            layer,
            diagnostics,
            ownership_metadata,
        }
    }

    pub fn layer(&self) -> &TypeLayer {
        &self.layer
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        &self.diagnostics
    }

    pub fn ownership_metadata(&self) -> &OwnershipMetadata {
        &self.ownership_metadata
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    pub fn into_layer(self) -> TypeLayer {
        self.layer
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnershipMetadata {
    anchors: Vec<AnchorMetadata>,
    edges: Vec<EdgeMetadata>,
    cycles: Vec<OwnershipCycleMetadata>,
    weak_observers: Vec<WeakObserverMetadata>,
    weak_fields: Vec<WeakFieldMetadata>,
    captures: Vec<CaptureMetadata>,
    temporaries: Vec<TemporaryMetadata>,
    release_eligibilities: Vec<ReleaseEligibilityMetadata>,
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

struct DeclarationTypeChecker<'a> {
    source: &'a SourceFile,
    graph: &'a ModuleGraph,
    layer: TypeLayer,
    diagnostics: DiagnosticBag,
    ownership_metadata: OwnershipMetadata,
}

impl<'a> DeclarationTypeChecker<'a> {
    fn new(source: &'a SourceFile, graph: &'a ModuleGraph, layer: TypeLayer) -> Self {
        Self {
            source,
            graph,
            layer,
            diagnostics: DiagnosticBag::new(),
            ownership_metadata: OwnershipMetadata::default(),
        }
    }

    fn into_result(self) -> TypeCheckResult {
        TypeCheckResult::with_ownership_metadata(
            self.layer,
            self.diagnostics,
            self.ownership_metadata,
        )
    }

    fn check(&mut self) {
        self.bind_builtin_symbol_types();
        self.bind_builtin_constraint_symbol_types();
        self.bind_named_type_definition_symbols();

        let Some(root) = self.graph.syntax().root() else {
            return;
        };

        self.check_node(root);
        self.check_decorators(root);
        self.check_control_flow(root, 0);
        self.check_initializer_types(root);
        self.check_enum_types(root);
        self.check_return_types(root, None);
        self.check_assignment_types(root);
        self.check_constraint_satisfies(root);
        self.check_function_stamps(root);
        self.check_semantic_rules(root);
        self.check_ownership_metadata(root);
    }

    fn describe_type_for_diagnostic(&self, ty: TypeId) -> String {
        let resolved = self.resolve_alias_type(ty);

        self.layer.table().describe(resolved)
    }

    fn bind_imported_symbol_types(&mut self, imported_types: &HashMap<SymbolId, ImportedType>) {
        for (symbol, imported_type) in imported_types {
            let ty = self.lower_imported_type(imported_type);
            self.layer.bind_symbol_type(*symbol, ty);
        }
    }

    fn lower_imported_type(&mut self, imported_type: &ImportedType) -> TypeId {
        match imported_type {
            ImportedType::Primitive(primitive) => self.layer.table().primitive(*primitive),

            ImportedType::NamedLocal { symbol } => self.layer.table_mut().intern_named(*symbol),

            ImportedType::Array { element } => {
                let element = self.lower_imported_type(element);
                self.layer.table_mut().intern_array(element)
            }

            ImportedType::FixedArray { element, size } => {
                let element = self.lower_imported_type(element);
                self.layer.table_mut().intern_fixed_array(element, *size)
            }

            ImportedType::Range { element } => {
                let element = self.lower_imported_type(element);
                self.layer.table_mut().intern_range(element)
            }

            ImportedType::Tuple { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_imported_type(element))
                    .collect::<Vec<_>>();

                self.layer.table_mut().intern_tuple(elements)
            }

            ImportedType::Union { members } => {
                let members = members
                    .iter()
                    .map(|member| self.lower_imported_type(member))
                    .collect::<Vec<_>>();

                self.layer.table_mut().intern_union(members)
            }

            ImportedType::Function {
                parameters,
                return_type,
            } => {
                let parameters = parameters
                    .iter()
                    .map(|parameter| {
                        let ty = self.lower_imported_type(parameter.ty());

                        if parameter.is_rest() {
                            return FunctionParameterType::rest(ty);
                        }

                        if parameter.has_default() {
                            return FunctionParameterType::with_default(ty);
                        }

                        FunctionParameterType::new(ty)
                    })
                    .collect::<Vec<_>>();

                let return_type = self.lower_imported_type(return_type);

                self.layer
                    .table_mut()
                    .intern_function(parameters, return_type)
            }
        }
    }
}

pub fn check_declaration_types(source: &SourceFile, graph: &ModuleGraph) -> TypeCheckResult {
    check_declaration_types_with_imports(source, graph, &HashMap::new())
}

pub fn check_declaration_types_with_imports(
    source: &SourceFile,
    graph: &ModuleGraph,
    imported_types: &HashMap<SymbolId, ImportedType>,
) -> TypeCheckResult {
    let lowering = lower_types(source, graph);

    let mut checker = DeclarationTypeChecker::new(source, graph, lowering.into_layer());
    checker.bind_imported_symbol_types(imported_types);
    checker.check();
    checker.into_result()
}

fn primitive_type_by_name(name: &str) -> Option<PrimitiveType> {
    match name {
        "null" => Some(PrimitiveType::Null),
        "bool" => Some(PrimitiveType::Bool),
        "int8" => Some(PrimitiveType::Int8),
        "int16" => Some(PrimitiveType::Int16),
        "int32" => Some(PrimitiveType::Int32),
        "int64" => Some(PrimitiveType::Int64),
        "uint8" => Some(PrimitiveType::Uint8),
        "uint16" => Some(PrimitiveType::Uint16),
        "uint32" => Some(PrimitiveType::Uint32),
        "uint64" => Some(PrimitiveType::Uint64),
        "float16" => Some(PrimitiveType::Float16),
        "float32" => Some(PrimitiveType::Float32),
        "float64" => Some(PrimitiveType::Float64),
        _ => None,
    }
}
