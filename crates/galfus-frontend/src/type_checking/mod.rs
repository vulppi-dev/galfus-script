#[cfg(test)]
mod tests;

mod access;
mod arrow_functions;
mod assignability;
mod assignments;
mod builtin_constraints;
mod calls;
mod constraints;
mod constraints_application;
mod control_flow;
mod declarations;
mod decorators;
mod diagnostics;
mod diagnostics_extra;
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
mod ownership_validation;
mod ranges;
mod returns;
mod semantics;
mod structs;
mod support;
mod variants;

use std::collections::HashMap;

use galfus_core::{DiagnosticBag, NodeId, SourceFile, SymbolId, TypeId};

use crate::{
    FunctionParameterType, ModuleGraph, PrimitiveType, SyntaxNodeKind, TypeLayer, lower_types,
};

mod model;
pub use model::*;

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
