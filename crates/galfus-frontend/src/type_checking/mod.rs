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

use crate::{
    ArraySize, FunctionParameterType, ModuleGraph, PrimitiveType, SyntaxNodeKind, TypeLayer,
    lower_types,
};

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
    imported_member_types: HashMap<ImportedMemberKey, TypeId>,
    imported_symbol_constraints: HashMap<SymbolId, LoweredImportedConstraint>,
    imported_path_constraints: HashMap<NodeId, LoweredImportedConstraint>,
    imported_symbol_choices: HashMap<SymbolId, LoweredImportedChoice>,
    imported_path_choices: HashMap<NodeId, LoweredImportedChoice>,
}

#[derive(Debug, Clone)]
struct LoweredImportedConstraint {
    name: String,
    generic_parameter_count: usize,
    fields: Vec<LoweredImportedConstraintMember>,
    functions: Vec<LoweredImportedConstraintMember>,
}

#[derive(Debug, Clone)]
struct LoweredImportedConstraintMember {
    name: String,
    ty: TypeId,
}

#[derive(Debug, Clone)]
struct LoweredImportedChoice {
    variants: Vec<LoweredImportedChoiceVariant>,
}

#[derive(Debug, Clone)]
struct LoweredImportedChoiceVariant {
    name: String,
    payload_types: Vec<TypeId>,
}

impl<'a> DeclarationTypeChecker<'a> {
    fn new(source: &'a SourceFile, graph: &'a ModuleGraph, layer: TypeLayer) -> Self {
        Self {
            source,
            graph,
            layer,
            diagnostics: DiagnosticBag::new(),
            ownership_metadata: OwnershipMetadata::default(),
            imported_member_types: HashMap::new(),
            imported_symbol_constraints: HashMap::new(),
            imported_path_constraints: HashMap::new(),
            imported_symbol_choices: HashMap::new(),
            imported_path_choices: HashMap::new(),
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
        self.check_expression_statements(root);
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

    fn bind_imported_path_types(&mut self, imported_types: &HashMap<NodeId, ImportedType>) {
        for (node, imported_type) in imported_types {
            let ty = self.lower_imported_type(imported_type);
            self.layer.bind_node_type(*node, ty);
        }
    }

    fn bind_imported_member_types(
        &mut self,
        imported_types: &HashMap<ImportedMemberKey, ImportedType>,
    ) {
        for (key, imported_type) in imported_types {
            let ty = self.lower_imported_type(imported_type);
            self.imported_member_types.insert(key.clone(), ty);
        }
    }

    fn bind_imported_symbol_constraints(
        &mut self,
        imported_constraints: &HashMap<SymbolId, ImportedConstraintSurface>,
    ) {
        for (symbol, imported_constraint) in imported_constraints {
            let constraint = self.lower_imported_constraint(imported_constraint);
            self.imported_symbol_constraints.insert(*symbol, constraint);
        }
    }

    fn bind_imported_path_constraints(
        &mut self,
        imported_constraints: &HashMap<NodeId, ImportedConstraintSurface>,
    ) {
        for (node, imported_constraint) in imported_constraints {
            let constraint = self.lower_imported_constraint(imported_constraint);
            self.imported_path_constraints.insert(*node, constraint);
        }
    }

    fn lower_imported_constraint(
        &mut self,
        imported_constraint: &ImportedConstraintSurface,
    ) -> LoweredImportedConstraint {
        LoweredImportedConstraint {
            name: imported_constraint.name().to_string(),
            generic_parameter_count: imported_constraint.generic_parameter_count(),
            fields: imported_constraint
                .fields()
                .iter()
                .map(|field| LoweredImportedConstraintMember {
                    name: field.name().to_string(),
                    ty: self.lower_imported_type(field.ty()),
                })
                .collect(),
            functions: imported_constraint
                .functions()
                .iter()
                .map(|function| LoweredImportedConstraintMember {
                    name: function.name().to_string(),
                    ty: self.lower_imported_type(function.ty()),
                })
                .collect(),
        }
    }

    fn bind_imported_symbol_choices(
        &mut self,
        imported_choices: &HashMap<SymbolId, ImportedChoiceSurface>,
    ) {
        for (symbol, imported_choice) in imported_choices {
            let choice = self.lower_imported_choice(imported_choice);
            self.imported_symbol_choices.insert(*symbol, choice);
        }
    }

    fn bind_imported_path_choices(
        &mut self,
        imported_choices: &HashMap<NodeId, ImportedChoiceSurface>,
    ) {
        for (node, imported_choice) in imported_choices {
            let choice = self.lower_imported_choice(imported_choice);
            self.imported_path_choices.insert(*node, choice);
        }
    }

    fn lower_imported_choice(
        &mut self,
        imported_choice: &ImportedChoiceSurface,
    ) -> LoweredImportedChoice {
        LoweredImportedChoice {
            variants: imported_choice
                .variants()
                .iter()
                .map(|variant| LoweredImportedChoiceVariant {
                    name: variant.name().to_string(),
                    payload_types: variant
                        .payload_types()
                        .iter()
                        .map(|ty| self.lower_imported_type(ty))
                        .collect(),
                })
                .collect(),
        }
    }

    fn lower_imported_type(&mut self, imported_type: &ImportedType) -> TypeId {
        match imported_type {
            ImportedType::Primitive(primitive) => self.layer.table().primitive(*primitive),

            ImportedType::NamedLocal { symbol } => self.layer.table_mut().intern_named(*symbol),

            ImportedType::SurfacePath { namespace, name } => self
                .layer
                .table_mut()
                .intern_path(*namespace, name.split("::").map(str::to_string).collect()),

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

    fn check_expression_statements(&mut self, node: NodeId) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        if syntax_node.kind() == SyntaxNodeKind::ExpressionStatement {
            if let Some(expression) = self.graph.syntax().child(node, 0) {
                self.infer_expression_type(expression);
            }
        }

        for child in syntax_node.children() {
            self.check_expression_statements(*child);
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
    let mut surface_types = ImportedSurfaceTypes::new();

    for (symbol, ty) in imported_types {
        surface_types.insert_symbol_type(*symbol, ty.clone());
    }

    check_declaration_types_with_surfaces(source, graph, &surface_types)
}

pub fn check_declaration_types_with_surfaces(
    source: &SourceFile,
    graph: &ModuleGraph,
    imported_types: &ImportedSurfaceTypes,
) -> TypeCheckResult {
    let lowering = lower_types(source, graph);

    let mut checker = DeclarationTypeChecker::new(source, graph, lowering.into_layer());
    checker.bind_imported_symbol_types(imported_types.symbol_types());
    checker.bind_imported_path_types(imported_types.path_types());
    checker.bind_imported_member_types(imported_types.member_types());
    checker.bind_imported_symbol_constraints(imported_types.symbol_constraints());
    checker.bind_imported_path_constraints(imported_types.path_constraints());
    checker.bind_imported_symbol_choices(imported_types.symbol_choices());
    checker.bind_imported_path_choices(imported_types.path_choices());
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
