use std::collections::HashMap;

use galfus_core::{DiagnosticBag, NodeId, SymbolId};

use crate::{ArraySize, PrimitiveType, TypeLayer};

pub use ownership_model::*;

mod ownership_model;

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
    SurfacePath {
        namespace: SymbolId,
        name: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedConstraintMember {
    name: String,
    ty: ImportedType,
}

impl ImportedConstraintMember {
    pub fn new(name: String, ty: ImportedType) -> Self {
        Self { name, ty }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn ty(&self) -> &ImportedType {
        &self.ty
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedConstraintSurface {
    name: String,
    generic_parameter_count: usize,
    fields: Vec<ImportedConstraintMember>,
    functions: Vec<ImportedConstraintMember>,
}

impl ImportedConstraintSurface {
    pub fn new(
        name: String,
        generic_parameter_count: usize,
        fields: Vec<ImportedConstraintMember>,
        functions: Vec<ImportedConstraintMember>,
    ) -> Self {
        Self {
            name,
            generic_parameter_count,
            fields,
            functions,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn generic_parameter_count(&self) -> usize {
        self.generic_parameter_count
    }

    pub fn fields(&self) -> &[ImportedConstraintMember] {
        self.fields.as_slice()
    }

    pub fn functions(&self) -> &[ImportedConstraintMember] {
        self.functions.as_slice()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedChoiceVariant {
    name: String,
    payload_types: Vec<ImportedType>,
}

impl ImportedChoiceVariant {
    pub fn new(name: String, payload_types: Vec<ImportedType>) -> Self {
        Self {
            name,
            payload_types,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn payload_types(&self) -> &[ImportedType] {
        self.payload_types.as_slice()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedChoiceSurface {
    name: String,
    variants: Vec<ImportedChoiceVariant>,
}

impl ImportedChoiceSurface {
    pub fn new(name: String, variants: Vec<ImportedChoiceVariant>) -> Self {
        Self { name, variants }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn variants(&self) -> &[ImportedChoiceVariant] {
        self.variants.as_slice()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportedMemberKey {
    namespace: SymbolId,
    owner: String,
    member: String,
}

impl ImportedMemberKey {
    pub fn new(namespace: SymbolId, owner: impl Into<String>, member: impl Into<String>) -> Self {
        Self {
            namespace,
            owner: owner.into(),
            member: member.into(),
        }
    }

    pub fn namespace(&self) -> SymbolId {
        self.namespace
    }

    pub fn owner(&self) -> &str {
        self.owner.as_str()
    }

    pub fn member(&self) -> &str {
        self.member.as_str()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportedSurfaceTypes {
    symbol_types: HashMap<SymbolId, ImportedType>,
    path_types: HashMap<NodeId, ImportedType>,
    member_types: HashMap<ImportedMemberKey, ImportedType>,
    symbol_constraints: HashMap<SymbolId, ImportedConstraintSurface>,
    path_constraints: HashMap<NodeId, ImportedConstraintSurface>,
    symbol_choices: HashMap<SymbolId, ImportedChoiceSurface>,
    path_choices: HashMap<NodeId, ImportedChoiceSurface>,
}

impl ImportedSurfaceTypes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn symbol_types(&self) -> &HashMap<SymbolId, ImportedType> {
        &self.symbol_types
    }

    pub fn path_types(&self) -> &HashMap<NodeId, ImportedType> {
        &self.path_types
    }

    pub fn member_types(&self) -> &HashMap<ImportedMemberKey, ImportedType> {
        &self.member_types
    }

    pub fn symbol_constraints(&self) -> &HashMap<SymbolId, ImportedConstraintSurface> {
        &self.symbol_constraints
    }

    pub fn path_constraints(&self) -> &HashMap<NodeId, ImportedConstraintSurface> {
        &self.path_constraints
    }

    pub fn symbol_choices(&self) -> &HashMap<SymbolId, ImportedChoiceSurface> {
        &self.symbol_choices
    }

    pub fn path_choices(&self) -> &HashMap<NodeId, ImportedChoiceSurface> {
        &self.path_choices
    }

    pub fn insert_symbol_type(&mut self, symbol: SymbolId, ty: ImportedType) {
        self.symbol_types.insert(symbol, ty);
    }

    pub fn insert_path_type(&mut self, node: NodeId, ty: ImportedType) {
        self.path_types.insert(node, ty);
    }

    pub fn insert_member_type(&mut self, key: ImportedMemberKey, ty: ImportedType) {
        self.member_types.insert(key, ty);
    }

    pub fn insert_symbol_constraint(
        &mut self,
        symbol: SymbolId,
        constraint: ImportedConstraintSurface,
    ) {
        self.symbol_constraints.insert(symbol, constraint);
    }

    pub fn insert_path_constraint(&mut self, node: NodeId, constraint: ImportedConstraintSurface) {
        self.path_constraints.insert(node, constraint);
    }

    pub fn insert_symbol_choice(&mut self, symbol: SymbolId, choice: ImportedChoiceSurface) {
        self.symbol_choices.insert(symbol, choice);
    }

    pub fn insert_path_choice(&mut self, node: NodeId, choice: ImportedChoiceSurface) {
        self.path_choices.insert(node, choice);
    }

    pub fn extend(&mut self, other: ImportedSurfaceTypes) {
        self.symbol_types.extend(other.symbol_types);
        self.path_types.extend(other.path_types);
        self.member_types.extend(other.member_types);
        self.symbol_constraints.extend(other.symbol_constraints);
        self.path_constraints.extend(other.path_constraints);
        self.symbol_choices.extend(other.symbol_choices);
        self.path_choices.extend(other.path_choices);
    }
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

    pub(super) fn with_ownership_metadata(
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
