#[cfg(test)]
mod tests;

use crate::{
    ArraySize, FunctionParameterType, ModuleGraph, PrimitiveType, SymbolKind, SyntaxNodeKind,
    TypeLayer,
};
use galfus_core::{NodeId, SourceFile, SymbolId, TypeId};

#[derive(Debug, Clone)]
pub struct TypeLoweringResult {
    layer: TypeLayer,
}

impl TypeLoweringResult {
    pub fn new(layer: TypeLayer) -> Self {
        Self { layer }
    }

    pub fn layer(&self) -> &TypeLayer {
        &self.layer
    }

    pub fn into_layer(self) -> TypeLayer {
        self.layer
    }
}

pub fn lower_types(source: &SourceFile, graph: &ModuleGraph) -> TypeLoweringResult {
    let mut lowerer = TypeLowerer::new(source, graph);
    lowerer.lower();

    TypeLoweringResult::new(lowerer.into_layer())
}

struct TypeLowerer<'a> {
    source: &'a SourceFile,
    graph: &'a ModuleGraph,
    layer: TypeLayer,
}

impl<'a> TypeLowerer<'a> {
    fn new(source: &'a SourceFile, graph: &'a ModuleGraph) -> Self {
        Self {
            source,
            graph,
            layer: TypeLayer::new(),
        }
    }

    fn into_layer(self) -> TypeLayer {
        self.layer
    }

    fn lower(&mut self) {
        let Some(root) = self.graph.syntax().root() else {
            return;
        };

        self.lower_types_in_node(root);
    }

    fn lower_types_in_node(&mut self, node: NodeId) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        if self.is_type_node_kind(syntax_node.kind()) {
            let ty = self.lower_type(node);
            self.layer.bind_node_type(node, ty);
            return;
        }

        for child in syntax_node.children() {
            self.lower_types_in_node(*child);
        }
    }

    fn lower_type(&mut self, node: NodeId) -> TypeId {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return self.error_type();
        };

        match syntax_node.kind() {
            SyntaxNodeKind::TypeNull => self.layer.table().primitive(PrimitiveType::Null),

            SyntaxNodeKind::NamedType => self.lower_named_type(node),

            SyntaxNodeKind::Path => self.lower_path_type(node),

            SyntaxNodeKind::ArrayType => self.lower_array_type(node),

            SyntaxNodeKind::FixedArrayType => self.lower_fixed_array_type(node),

            SyntaxNodeKind::TupleType => self.lower_tuple_type(node),

            SyntaxNodeKind::GroupedType => self.lower_grouped_type(node),

            SyntaxNodeKind::UnionType => self.lower_union_type(node),

            SyntaxNodeKind::GenericType => self.lower_generic_type(node),

            SyntaxNodeKind::FunctionType => self.lower_function_type(node),

            _ => self.error_type(),
        }
    }

    fn lower_named_type(&mut self, node: NodeId) -> TypeId {
        let Some(identifier) = self
            .graph
            .syntax()
            .first_child_of_kind(node, SyntaxNodeKind::Identifier)
        else {
            return self.error_type();
        };

        let name = self.node_text(identifier);

        if name == "int" {
            let members = vec![
                self.layer.table().primitive(PrimitiveType::Int8),
                self.layer.table().primitive(PrimitiveType::Int16),
                self.layer.table().primitive(PrimitiveType::Int32),
                self.layer.table().primitive(PrimitiveType::Int64),
                self.layer.table().primitive(PrimitiveType::Uint8),
                self.layer.table().primitive(PrimitiveType::Uint16),
                self.layer.table().primitive(PrimitiveType::Uint32),
                self.layer.table().primitive(PrimitiveType::Uint64),
            ];
            return self.layer.table_mut().intern_union(members);
        }

        if name == "float" {
            let members = vec![
                self.layer.table().primitive(PrimitiveType::Float16),
                self.layer.table().primitive(PrimitiveType::Float32),
                self.layer.table().primitive(PrimitiveType::Float64),
            ];
            return self.layer.table_mut().intern_union(members);
        }

        if let Some(primitive) = primitive_type_by_name(name.as_str()) {
            return self.layer.table().primitive(primitive);
        }

        let Some(resolution) = self.graph.resolution() else {
            return self.error_type();
        };

        let Some(symbol) = resolution.type_reference_symbol(node) else {
            return self.error_type();
        };

        self.lower_symbol_type(symbol)
    }

    fn lower_path_type(&mut self, node: NodeId) -> TypeId {
        let Some(resolution) = self.graph.resolution() else {
            return self.error_type();
        };

        if let Some(symbol) = resolution.type_path_reference_symbol(node) {
            return self.lower_symbol_type(symbol);
        }

        let Some(root_symbol) = resolution.type_reference_symbol(node) else {
            return self.error_type();
        };

        let Some(symbol_data) = resolution.symbol(root_symbol) else {
            return self.error_type();
        };

        if symbol_data.kind() == SymbolKind::ImportNamespace {
            let segments = self.path_segments(node);

            if segments.len() <= 1 {
                return self.error_type();
            }

            return self
                .layer
                .table_mut()
                .intern_path(root_symbol, segments.into_iter().skip(1).collect());
        }

        self.lower_symbol_type(root_symbol)
    }

    fn lower_symbol_type(&mut self, symbol: SymbolId) -> TypeId {
        let Some(resolution) = self.graph.resolution() else {
            return self.error_type();
        };

        let Some(symbol_data) = resolution.symbol(symbol) else {
            return self.error_type();
        };

        if symbol_data.kind() == SymbolKind::BuiltinType {
            if symbol_data.name() == "int" {
                let members = vec![
                    self.layer.table().primitive(PrimitiveType::Int8),
                    self.layer.table().primitive(PrimitiveType::Int16),
                    self.layer.table().primitive(PrimitiveType::Int32),
                    self.layer.table().primitive(PrimitiveType::Int64),
                    self.layer.table().primitive(PrimitiveType::Uint8),
                    self.layer.table().primitive(PrimitiveType::Uint16),
                    self.layer.table().primitive(PrimitiveType::Uint32),
                    self.layer.table().primitive(PrimitiveType::Uint64),
                ];
                return self.layer.table_mut().intern_union(members);
            }
            if symbol_data.name() == "float" {
                let members = vec![
                    self.layer.table().primitive(PrimitiveType::Float16),
                    self.layer.table().primitive(PrimitiveType::Float32),
                    self.layer.table().primitive(PrimitiveType::Float64),
                ];
                return self.layer.table_mut().intern_union(members);
            }
            if let Some(primitive) = primitive_type_by_name(symbol_data.name()) {
                return self.layer.table().primitive(primitive);
            }

            return self.error_type();
        }

        if symbol_data.kind() == SymbolKind::GenericParameter {
            return self.layer.table_mut().intern_generic_parameter(symbol);
        }

        self.layer.table_mut().intern_named(symbol)
    }

    fn lower_array_type(&mut self, node: NodeId) -> TypeId {
        let Some(element) = self.graph.syntax().child(node, 0) else {
            return self.error_type();
        };

        let element = self.lower_type(element);
        self.layer.table_mut().intern_array(element)
    }

    fn lower_fixed_array_type(&mut self, node: NodeId) -> TypeId {
        let Some(element) = self.graph.syntax().child(node, 0) else {
            return self.error_type();
        };

        let Some(size_node) = self.graph.syntax().child(node, 1) else {
            return self.error_type();
        };

        let element = self.lower_type(element);
        let size = self.lower_array_size(size_node);

        self.layer.table_mut().intern_fixed_array(element, size)
    }

    fn lower_array_size(&self, node: NodeId) -> ArraySize {
        let Some(integer) = self
            .graph
            .syntax()
            .first_child_of_kind(node, SyntaxNodeKind::IntegerLiteral)
        else {
            return ArraySize::Unknown;
        };

        let text = self.node_text(integer);

        match text.parse::<u64>() {
            Ok(value) => ArraySize::Known(value),
            Err(_) => ArraySize::Unknown,
        }
    }

    fn lower_tuple_type(&mut self, node: NodeId) -> TypeId {
        let elements = self
            .graph
            .syntax()
            .node(node)
            .map(|node| node.children().to_vec())
            .unwrap_or_default()
            .into_iter()
            .map(|child| self.lower_type(child))
            .collect::<Vec<_>>();

        self.layer.table_mut().intern_tuple(elements)
    }

    fn lower_grouped_type(&mut self, node: NodeId) -> TypeId {
        let Some(inner) = self.graph.syntax().child(node, 0) else {
            return self.error_type();
        };

        self.lower_type(inner)
    }

    fn lower_union_type(&mut self, node: NodeId) -> TypeId {
        let members = self
            .graph
            .syntax()
            .node(node)
            .map(|node| node.children().to_vec())
            .unwrap_or_default()
            .into_iter()
            .map(|child| self.lower_type(child))
            .collect::<Vec<_>>();

        self.layer.table_mut().intern_union(members)
    }

    fn lower_generic_type(&mut self, node: NodeId) -> TypeId {
        let Some(base) = self.graph.syntax().child(node, 0) else {
            return self.error_type();
        };

        let Some(arguments) = self.graph.syntax().child(node, 1) else {
            return self.error_type();
        };

        let base = self.lower_type(base);

        let arguments = self
            .graph
            .syntax()
            .node(arguments)
            .map(|node| node.children().to_vec())
            .unwrap_or_default()
            .into_iter()
            .map(|argument| self.lower_type(argument))
            .collect::<Vec<_>>();

        self.layer
            .table_mut()
            .intern_generic_instance(base, arguments)
    }

    fn lower_function_type(&mut self, node: NodeId) -> TypeId {
        let Some(parameters_node) = self.graph.syntax().child(node, 0) else {
            return self.error_type();
        };

        let Some(return_type_node) = self.graph.syntax().child(node, 1) else {
            return self.error_type();
        };

        let parameters = self
            .graph
            .syntax()
            .node(parameters_node)
            .map(|node| node.children().to_vec())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|parameter| self.lower_function_type_parameter(parameter))
            .collect::<Vec<_>>();

        let return_type = self.lower_type(return_type_node);

        self.layer
            .table_mut()
            .intern_function(parameters, return_type)
    }

    fn lower_function_type_parameter(
        &mut self,
        parameter: NodeId,
    ) -> Option<FunctionParameterType> {
        let type_node = if self.is_type_node(parameter) {
            parameter
        } else {
            self.first_type_child(parameter)?
        };

        Some(FunctionParameterType::new(self.lower_type(type_node)))
    }

    fn first_type_child(&self, node: NodeId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        for child in syntax_node.children() {
            if self.is_type_node(*child) {
                return Some(*child);
            }

            if let Some(found) = self.first_type_child(*child) {
                return Some(found);
            }
        }

        None
    }

    fn is_type_node(&self, node: NodeId) -> bool {
        self.graph
            .syntax()
            .node(node)
            .map(|node| self.is_type_node_kind(node.kind()))
            .unwrap_or(false)
    }

    fn is_type_node_kind(&self, kind: SyntaxNodeKind) -> bool {
        matches!(
            kind,
            SyntaxNodeKind::TypeNull
                | SyntaxNodeKind::NamedType
                | SyntaxNodeKind::Path
                | SyntaxNodeKind::ArrayType
                | SyntaxNodeKind::FixedArrayType
                | SyntaxNodeKind::TupleType
                | SyntaxNodeKind::GroupedType
                | SyntaxNodeKind::UnionType
                | SyntaxNodeKind::GenericType
                | SyntaxNodeKind::FunctionType
        )
    }

    fn path_segments(&self, path: NodeId) -> Vec<String> {
        let Some(path_node) = self.graph.syntax().node(path) else {
            return Vec::new();
        };

        path_node
            .children()
            .iter()
            .filter_map(|child| {
                let child_node = self.graph.syntax().node(*child)?;

                if child_node.kind() != SyntaxNodeKind::Identifier {
                    return None;
                }

                Some(self.node_text(*child))
            })
            .collect()
    }

    fn node_text(&self, node: NodeId) -> String {
        let Some(node) = self.graph.syntax().node(node) else {
            return String::new();
        };

        self.source.slice(node.span()).unwrap_or("").to_string()
    }

    fn error_type(&mut self) -> TypeId {
        self.layer.table_mut().error()
    }
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
