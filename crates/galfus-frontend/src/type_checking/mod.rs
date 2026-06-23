#[cfg(test)]
mod tests;

mod access;
mod assignability;
mod assignments;
mod builtin_constraints;
mod calls;
mod constraints;
mod control_flow;
mod declarations;
mod diagnostics;
mod expressions;
mod inferred_structs;
mod initializers;
mod instanceof;
mod literals;
mod matches;
mod operators;
mod ranges;
mod returns;
mod structs;
mod support;
mod variants;

use galfus_core::{DiagnosticBag, SourceFile, TypeId};

use crate::{ModuleGraph, PrimitiveType, TypeLayer, lower_types};

#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    layer: TypeLayer,
    diagnostics: DiagnosticBag,
}

impl TypeCheckResult {
    pub fn new(layer: TypeLayer, diagnostics: DiagnosticBag) -> Self {
        Self { layer, diagnostics }
    }

    pub fn layer(&self) -> &TypeLayer {
        &self.layer
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        &self.diagnostics
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    pub fn into_layer(self) -> TypeLayer {
        self.layer
    }
}

struct DeclarationTypeChecker<'a> {
    source: &'a SourceFile,
    graph: &'a ModuleGraph,
    layer: TypeLayer,
    diagnostics: DiagnosticBag,
}

impl<'a> DeclarationTypeChecker<'a> {
    fn new(source: &'a SourceFile, graph: &'a ModuleGraph, layer: TypeLayer) -> Self {
        Self {
            source,
            graph,
            layer,
            diagnostics: DiagnosticBag::new(),
        }
    }

    fn into_result(self) -> TypeCheckResult {
        TypeCheckResult::new(self.layer, self.diagnostics)
    }

    fn check(&mut self) {
        self.bind_builtin_symbol_types();
        self.bind_builtin_constraint_symbol_types();
        self.bind_named_type_definition_symbols();

        let Some(root) = self.graph.syntax().root() else {
            return;
        };

        self.check_node(root);
        self.check_control_flow(root, 0);
        self.check_initializer_types(root);
        self.check_return_types(root, None);
        self.check_assignment_types(root);
        self.check_constraint_satisfies(root);
    }

    fn describe_type_for_diagnostic(&self, ty: TypeId) -> String {
        let resolved = self.resolve_alias_type(ty);

        self.layer.table().describe(resolved)
    }
}

pub fn check_declaration_types(source: &SourceFile, graph: &ModuleGraph) -> TypeCheckResult {
    let lowering = lower_types(source, graph);

    let mut checker = DeclarationTypeChecker::new(source, graph, lowering.into_layer());
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
