#[cfg(test)]
mod tests;

use std::collections::HashMap;

use galfus_core::{SymbolId, TypeId};

use crate::{
    ImportedChoiceSurface, ImportedChoiceVariant, ImportedConstraintMember,
    ImportedConstraintSurface, ImportedFunctionParameterType, ImportedMemberKey,
    ImportedSurfaceTypes, ImportedType, ModuleGraph, SymbolKind, SyntaxNodeKind, TypeCheckResult,
    TypeKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSurface {
    exports: Vec<ModuleSurfaceExport>,
    exports_by_name: HashMap<String, usize>,
}

impl ModuleSurface {
    pub fn new(exports: Vec<ModuleSurfaceExport>) -> Self {
        let exports_by_name = exports
            .iter()
            .enumerate()
            .map(|(index, export)| (export.name().to_string(), index))
            .collect();

        Self {
            exports,
            exports_by_name,
        }
    }

    pub fn exports(&self) -> &[ModuleSurfaceExport] {
        self.exports.as_slice()
    }

    pub fn export(&self, name: &str) -> Option<&ModuleSurfaceExport> {
        self.exports_by_name
            .get(name)
            .and_then(|index| self.exports.get(*index))
    }

    pub fn imported_type_for_export(
        &self,
        local_symbol: SymbolId,
        name: &str,
    ) -> Option<ImportedType> {
        let export = self.export(name)?;

        if export.kind().is_nominal_surface_type() {
            return Some(ImportedType::NamedLocal {
                symbol: local_symbol,
            });
        }

        export.ty().cloned()
    }

    pub fn imported_path_type_for_export(
        &self,
        namespace: SymbolId,
        name: &str,
    ) -> Option<ImportedType> {
        if let Some(export) = self.export(name) {
            if export.kind().is_nominal_surface_type() {
                return Some(ImportedType::SurfacePath {
                    namespace,
                    name: name.to_string(),
                });
            }

            return export.ty().cloned();
        }

        let (owner_name, member_name) = name.rsplit_once("::")?;
        let owner = self.export(owner_name)?;
        let member = owner
            .members()
            .iter()
            .find(|member| member.name() == member_name)?;

        match member.kind() {
            SymbolKind::EnumVariant => Some(ImportedType::SurfacePath {
                namespace,
                name: owner_name.to_string(),
            }),

            SymbolKind::ChoiceVariant => {
                let owner_type = ImportedType::SurfacePath {
                    namespace,
                    name: owner_name.to_string(),
                };

                if member.payload_types().is_empty() {
                    return Some(owner_type);
                }

                let parameters = member
                    .payload_types()
                    .iter()
                    .cloned()
                    .map(ImportedFunctionParameterType::new)
                    .collect();

                Some(ImportedType::Function {
                    parameters,
                    return_type: Box::new(owner_type),
                })
            }

            _ => member.ty().cloned(),
        }
    }

    pub fn imported_member_path_type_for_named_export(
        &self,
        local_symbol: SymbolId,
        owner_name: &str,
        member_name: &str,
    ) -> Option<ImportedType> {
        let owner = self.export(owner_name)?;
        let member = owner
            .members()
            .iter()
            .find(|member| member.name() == member_name)?;

        match member.kind() {
            SymbolKind::EnumVariant => Some(ImportedType::NamedLocal {
                symbol: local_symbol,
            }),

            SymbolKind::ChoiceVariant => {
                let owner_type = ImportedType::NamedLocal {
                    symbol: local_symbol,
                };

                if member.payload_types().is_empty() {
                    return Some(owner_type);
                }

                let parameters = member
                    .payload_types()
                    .iter()
                    .cloned()
                    .map(ImportedFunctionParameterType::new)
                    .collect();

                Some(ImportedType::Function {
                    parameters,
                    return_type: Box::new(owner_type),
                })
            }

            _ => member.ty().cloned(),
        }
    }

    pub fn imported_constraint_for_export(&self, name: &str) -> Option<ImportedConstraintSurface> {
        let export = self.export(name)?;

        if export.kind() != SymbolKind::Constraint {
            return None;
        }

        Some(export.imported_constraint_surface())
    }

    pub fn imported_choice_for_export(&self, name: &str) -> Option<ImportedChoiceSurface> {
        let export = self.export(name)?;

        if export.kind() != SymbolKind::Choice {
            return None;
        }

        Some(export.imported_choice_surface())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSurfaceExport {
    name: String,
    kind: SymbolKind,
    ty: Option<ImportedType>,
    members: Vec<ModuleSurfaceMember>,
    generic_parameter_count: usize,
}

impl ModuleSurfaceExport {
    pub fn new(name: String, kind: SymbolKind, ty: Option<ImportedType>) -> Self {
        Self::with_members(name, kind, ty, Vec::new(), 0)
    }

    pub fn with_members(
        name: String,
        kind: SymbolKind,
        ty: Option<ImportedType>,
        members: Vec<ModuleSurfaceMember>,
        generic_parameter_count: usize,
    ) -> Self {
        Self {
            name,
            kind,
            ty,
            members,
            generic_parameter_count,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn kind(&self) -> SymbolKind {
        self.kind
    }

    pub fn ty(&self) -> Option<&ImportedType> {
        self.ty.as_ref()
    }

    pub fn members(&self) -> &[ModuleSurfaceMember] {
        self.members.as_slice()
    }

    pub fn generic_parameter_count(&self) -> usize {
        self.generic_parameter_count
    }

    fn imported_constraint_surface(&self) -> ImportedConstraintSurface {
        let fields = self
            .members
            .iter()
            .filter_map(|member| {
                if member.kind() != SymbolKind::ConstraintField {
                    return None;
                }

                Some(ImportedConstraintMember::new(
                    member.name().to_string(),
                    member.ty()?.clone(),
                ))
            })
            .collect();

        let functions = self
            .members
            .iter()
            .filter_map(|member| {
                if member.kind() != SymbolKind::ConstraintFunction {
                    return None;
                }

                Some(ImportedConstraintMember::new(
                    member.name().to_string(),
                    member.ty()?.clone(),
                ))
            })
            .collect();

        ImportedConstraintSurface::new(
            self.name.clone(),
            self.generic_parameter_count,
            fields,
            functions,
        )
    }

    fn imported_choice_surface(&self) -> ImportedChoiceSurface {
        let variants = self
            .members
            .iter()
            .filter_map(|member| {
                if member.kind() != SymbolKind::ChoiceVariant {
                    return None;
                }

                Some(ImportedChoiceVariant::new(
                    member.name().to_string(),
                    member.payload_types().to_vec(),
                ))
            })
            .collect();

        ImportedChoiceSurface::new(self.name.clone(), variants)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSurfaceMember {
    name: String,
    kind: SymbolKind,
    ty: Option<ImportedType>,
    payload_types: Vec<ImportedType>,
}

impl ModuleSurfaceMember {
    pub fn new(name: String, kind: SymbolKind, ty: Option<ImportedType>) -> Self {
        Self {
            name,
            kind,
            ty,
            payload_types: Vec::new(),
        }
    }

    pub fn with_payload(name: String, kind: SymbolKind, payload_types: Vec<ImportedType>) -> Self {
        Self {
            name,
            kind,
            ty: None,
            payload_types,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn kind(&self) -> SymbolKind {
        self.kind
    }

    pub fn ty(&self) -> Option<&ImportedType> {
        self.ty.as_ref()
    }

    pub fn payload_types(&self) -> &[ImportedType] {
        self.payload_types.as_slice()
    }
}

pub fn build_module_surface(graph: &ModuleGraph, type_result: &TypeCheckResult) -> ModuleSurface {
    let Some(resolution) = graph.resolution() else {
        return ModuleSurface::new(Vec::new());
    };

    let exports = resolution
        .exports()
        .iter()
        .map(|export| {
            let ty = if export.kind().is_nominal_surface_type() {
                None
            } else {
                type_result
                    .layer()
                    .symbol_type(export.symbol())
                    .and_then(|ty| transport_type(type_result, ty))
            };

            let members = surface_members_for_export(graph, type_result, export.symbol());
            let generic_parameter_count =
                surface_generic_parameter_count(graph, export.symbol(), export.kind());

            ModuleSurfaceExport::with_members(
                export.name().to_string(),
                export.kind(),
                ty,
                members,
                generic_parameter_count,
            )
        })
        .collect();

    ModuleSurface::new(exports)
}

pub fn imported_surface_types_for_namespace(
    surface: &ModuleSurface,
    namespace: SymbolId,
) -> ImportedSurfaceTypes {
    let mut imported_types = ImportedSurfaceTypes::new();

    for export in surface.exports() {
        for member in export.members() {
            if let Some(ty) = member.ty() {
                imported_types.insert_member_type(
                    ImportedMemberKey::new(namespace, export.name(), member.name()),
                    ty.clone(),
                );
            }
        }
    }

    imported_types
}

pub fn imported_surface_types_for_named_export(
    surface: &ModuleSurface,
    local_symbol: SymbolId,
    name: &str,
) -> ImportedSurfaceTypes {
    let mut imported_types = ImportedSurfaceTypes::new();
    let Some(export) = surface.export(name) else {
        return imported_types;
    };

    for member in export.members() {
        if let Some(ty) = member.ty() {
            imported_types.insert_member_type(
                ImportedMemberKey::new(local_symbol, "", member.name()),
                ty.clone(),
            );
        }
    }

    imported_types
}

fn surface_members_for_export(
    graph: &ModuleGraph,
    type_result: &TypeCheckResult,
    symbol: SymbolId,
) -> Vec<ModuleSurfaceMember> {
    let Some(resolution) = graph.resolution() else {
        return Vec::new();
    };

    let Some(member_scope) = resolution.member_scope(symbol) else {
        return Vec::new();
    };

    let Some(scope) = resolution.scope(member_scope) else {
        return Vec::new();
    };

    scope
        .symbols()
        .iter()
        .filter_map(|(name, member_symbol)| {
            let member = resolution.symbol(*member_symbol)?;

            match member.kind() {
                SymbolKind::StructField | SymbolKind::ConstraintField => {
                    let ty = type_result
                        .layer()
                        .symbol_type(*member_symbol)
                        .and_then(|ty| transport_type(type_result, ty))?;

                    Some(ModuleSurfaceMember::new(
                        name.to_string(),
                        member.kind(),
                        Some(ty),
                    ))
                }

                SymbolKind::ConstraintFunction => {
                    let ty = type_result
                        .layer()
                        .symbol_type(*member_symbol)
                        .and_then(|ty| transport_type(type_result, ty))?;

                    Some(ModuleSurfaceMember::new(
                        name.to_string(),
                        member.kind(),
                        Some(ty),
                    ))
                }

                SymbolKind::EnumVariant => Some(ModuleSurfaceMember::new(
                    name.to_string(),
                    member.kind(),
                    None,
                )),

                SymbolKind::ChoiceVariant => {
                    let payload_types =
                        choice_payload_types(graph, type_result, member.declaration())?;

                    Some(ModuleSurfaceMember::with_payload(
                        name.to_string(),
                        member.kind(),
                        payload_types,
                    ))
                }

                _ => None,
            }
        })
        .collect()
}

fn surface_generic_parameter_count(
    graph: &ModuleGraph,
    symbol: SymbolId,
    kind: SymbolKind,
) -> usize {
    if kind != SymbolKind::Constraint {
        return 0;
    }

    let Some(resolution) = graph.resolution() else {
        return 0;
    };

    let Some(member_scope) = resolution.member_scope(symbol) else {
        return 0;
    };

    let Some(scope) = resolution.scope(member_scope) else {
        return 0;
    };

    let Some(owner) = scope.owner() else {
        return 0;
    };

    declaration_symbols_in_node(graph, owner, SymbolKind::GenericParameter)
}

fn declaration_symbols_in_node(
    graph: &ModuleGraph,
    node: galfus_core::NodeId,
    kind: SymbolKind,
) -> usize {
    let Some(syntax_node) = graph.syntax().node(node) else {
        return 0;
    };

    let current = graph
        .resolution()
        .and_then(|resolution| resolution.declaration_symbol(node))
        .and_then(|symbol| graph.resolution()?.symbol(symbol))
        .is_some_and(|symbol| symbol.kind() == kind) as usize;

    current
        + syntax_node
            .children()
            .iter()
            .map(|child| declaration_symbols_in_node(graph, *child, kind))
            .sum::<usize>()
}

fn choice_payload_types(
    graph: &ModuleGraph,
    type_result: &TypeCheckResult,
    declaration: galfus_core::NodeId,
) -> Option<Vec<ImportedType>> {
    let root = graph.syntax().root()?;
    let variant = find_parent_choice_variant(graph, root, declaration)?;
    let payload = find_descendant_of_kind(graph, variant, SyntaxNodeKind::ChoicePayload)?;
    let payload_node = graph.syntax().node(payload)?;

    payload_node
        .children()
        .iter()
        .map(|child| {
            let type_node = first_type_child(graph, *child).unwrap_or(*child);
            let ty = type_result.layer().node_type(type_node)?;

            transport_type(type_result, ty)
        })
        .collect()
}

fn find_parent_choice_variant(
    graph: &ModuleGraph,
    node: galfus_core::NodeId,
    declaration: galfus_core::NodeId,
) -> Option<galfus_core::NodeId> {
    let syntax_node = graph.syntax().node(node)?;

    if syntax_node.kind() == SyntaxNodeKind::ChoiceVariant
        && graph
            .syntax()
            .first_child_of_kind(node, SyntaxNodeKind::Identifier)
            == Some(declaration)
    {
        return Some(node);
    }

    for child in syntax_node.children() {
        if let Some(found) = find_parent_choice_variant(graph, *child, declaration) {
            return Some(found);
        }
    }

    None
}

fn find_descendant_of_kind(
    graph: &ModuleGraph,
    node: galfus_core::NodeId,
    kind: SyntaxNodeKind,
) -> Option<galfus_core::NodeId> {
    let syntax_node = graph.syntax().node(node)?;

    for child in syntax_node.children() {
        let child_node = graph.syntax().node(*child)?;

        if child_node.kind() == kind {
            return Some(*child);
        }

        if let Some(found) = find_descendant_of_kind(graph, *child, kind) {
            return Some(found);
        }
    }

    None
}

fn first_type_child(graph: &ModuleGraph, node: galfus_core::NodeId) -> Option<galfus_core::NodeId> {
    let syntax_node = graph.syntax().node(node)?;

    if syntax_node.kind().is_type() {
        return Some(node);
    }

    syntax_node.children().iter().copied().find(|child| {
        graph
            .syntax()
            .node(*child)
            .is_some_and(|node| node.kind().is_type())
    })
}

fn transport_type(result: &TypeCheckResult, ty: TypeId) -> Option<ImportedType> {
    match result.layer().table().kind(ty).cloned()? {
        TypeKind::Primitive(primitive) => Some(ImportedType::Primitive(primitive)),

        TypeKind::Array { element } => Some(ImportedType::Array {
            element: Box::new(transport_type(result, element)?),
        }),

        TypeKind::FixedArray { element, size } => Some(ImportedType::FixedArray {
            element: Box::new(transport_type(result, element)?),
            size,
        }),

        TypeKind::Range { element } => Some(ImportedType::Range {
            element: Box::new(transport_type(result, element)?),
        }),

        TypeKind::Tuple { elements } => {
            let elements = elements
                .into_iter()
                .map(|element| transport_type(result, element))
                .collect::<Option<Vec<_>>>()?;

            Some(ImportedType::Tuple { elements })
        }

        TypeKind::Union { members } => {
            let members = members
                .into_iter()
                .map(|member| transport_type(result, member))
                .collect::<Option<Vec<_>>>()?;

            Some(ImportedType::Union { members })
        }

        TypeKind::Function(function) => {
            let parameters = function
                .parameters()
                .iter()
                .map(|parameter| {
                    let ty = transport_type(result, parameter.ty())?;

                    if parameter.is_rest() {
                        return Some(ImportedFunctionParameterType::rest(ty));
                    }

                    if parameter.has_default() {
                        return Some(ImportedFunctionParameterType::with_default(ty));
                    }

                    Some(ImportedFunctionParameterType::new(ty))
                })
                .collect::<Option<Vec<_>>>()?;

            let return_type = Box::new(transport_type(result, function.return_type())?);

            Some(ImportedType::Function {
                parameters,
                return_type,
            })
        }

        _ => None,
    }
}

impl SymbolKind {
    pub fn is_type_definition(self) -> bool {
        matches!(
            self,
            Self::Struct | Self::Enum | Self::Choice | Self::Constraint | Self::TypeAlias
        )
    }

    pub fn is_nominal_surface_type(self) -> bool {
        matches!(
            self,
            Self::Struct | Self::Enum | Self::Choice | Self::Constraint
        )
    }
}
