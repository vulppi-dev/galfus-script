use std::collections::HashMap;

use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{FunctionParameterType, SymbolKind, SyntaxNodeKind, TypeKind};

use super::DeclarationTypeChecker;

#[derive(Debug, Clone)]
struct ConstraintFieldInfo {
    name: String,
    ty: TypeId,
}

#[derive(Debug, Clone)]
struct StructFieldInfo {
    node: NodeId,
    name: String,
    ty: TypeId,
}

#[derive(Debug, Clone)]
struct ConstraintFunctionInfo {
    name: String,
    ty: TypeId,
}

#[derive(Debug, Clone)]
struct StructFunctionInfo {
    node: NodeId,
    name: String,
    ty: TypeId,
}

pub(super) type TypeSubstitution = HashMap<SymbolId, TypeId>;

#[derive(Debug, Clone)]
pub(super) struct ConstraintApplication {
    pub(super) symbol: SymbolId,
    pub(super) substitution: TypeSubstitution,
}

#[derive(Debug, Clone)]
enum ConstraintApplicationError {
    InvalidTarget,
    GenericArgumentCountMismatch {
        constraint_name: String,
        expected: usize,
        actual: usize,
    },
}

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn check_constraint_satisfies(&mut self, node: NodeId) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        match syntax_node.kind() {
            SyntaxNodeKind::StructItem => {
                self.check_struct_satisfies(node);
            }
            SyntaxNodeKind::GenericParameterConstraint => {
                self.check_generic_parameter_constraint(node);
            }
            _ => {}
        }

        let children = syntax_node.children().to_vec();

        for child in children {
            self.check_constraint_satisfies(child);
        }
    }

    fn check_struct_satisfies(&mut self, struct_item: NodeId) {
        let Some(satisfies) = self
            .graph
            .syntax()
            .first_child_of_kind(struct_item, SyntaxNodeKind::SatisfiesClause)
        else {
            return;
        };

        let Some((struct_symbol, struct_name)) = self.struct_item_symbol(struct_item) else {
            return;
        };

        let struct_fields = self.struct_satisfies_fields(struct_symbol);

        let struct_functions = self.struct_satisfies_functions(struct_name.as_str());

        let constraints = self
            .graph
            .syntax()
            .node(satisfies)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        for constraint_type in constraints {
            self.check_single_satisfies_constraint(
                struct_item,
                constraint_type,
                struct_name.as_str(),
                &struct_fields,
                &struct_functions,
            );
        }
    }

    fn check_single_satisfies_constraint(
        &mut self,
        struct_item: NodeId,
        constraint_type: NodeId,
        struct_name: &str,
        struct_fields: &[StructFieldInfo],
        struct_functions: &[StructFunctionInfo],
    ) {
        let target_name = self.node_text(constraint_type);

        let application = match self.constraint_application(constraint_type) {
            Ok(application) => application,

            Err(ConstraintApplicationError::InvalidTarget) => {
                self.report_invalid_satisfies_target(constraint_type, target_name.as_str());
                return;
            }

            Err(ConstraintApplicationError::GenericArgumentCountMismatch {
                constraint_name,
                expected,
                actual,
            }) => {
                self.report_constraint_generic_argument_count_mismatch(
                    constraint_type,
                    constraint_name.as_str(),
                    expected,
                    actual,
                );
                return;
            }
        };

        let constraint_symbol = application.symbol;
        let substitution = application.substitution;

        let Some(constraint_name) = self.symbol_name(constraint_symbol) else {
            return;
        };

        let constraint_fields = self.constraint_fields(constraint_symbol);

        for constraint_field in constraint_fields {
            let Some(struct_field) = struct_fields
                .iter()
                .find(|field| field.name == constraint_field.name)
            else {
                self.report_missing_constraint_field(
                    struct_item,
                    struct_name,
                    constraint_name.as_str(),
                    constraint_field.name.as_str(),
                );
                continue;
            };

            let expected = self.substitute_type(constraint_field.ty, &substitution);

            if self.is_assignable(expected, struct_field.ty) {
                continue;
            }

            self.report_constraint_field_type_mismatch(
                struct_field.node,
                struct_name,
                constraint_name.as_str(),
                constraint_field.name.as_str(),
                expected,
                struct_field.ty,
            );
        }

        let constraint_functions = self.constraint_functions(constraint_symbol);

        for constraint_function in constraint_functions {
            let Some(struct_function) = struct_functions
                .iter()
                .find(|function| function.name == constraint_function.name)
            else {
                self.report_missing_constraint_function(
                    struct_item,
                    struct_name,
                    constraint_name.as_str(),
                    constraint_function.name.as_str(),
                );
                continue;
            };

            let expected = self.substitute_type(constraint_function.ty, &substitution);

            if self.is_assignable(expected, struct_function.ty) {
                continue;
            }

            self.report_constraint_function_type_mismatch(
                struct_function.node,
                struct_name,
                constraint_name.as_str(),
                constraint_function.name.as_str(),
                expected,
                struct_function.ty,
            );
        }
    }

    fn struct_item_symbol(&self, struct_item: NodeId) -> Option<(SymbolId, String)> {
        let name_node = self
            .graph
            .syntax()
            .first_child_of_kind(struct_item, SyntaxNodeKind::Identifier)?;

        let struct_name = self.node_text(name_node);
        let resolution = self.graph.resolution()?;

        let symbol = resolution
            .symbols()
            .iter()
            .find(|symbol| symbol.name() == struct_name && symbol.kind() == SymbolKind::Struct)
            .map(|symbol| symbol.id())?;

        Some((symbol, struct_name))
    }

    pub(super) fn symbol_name(&self, symbol: SymbolId) -> Option<String> {
        let resolution = self.graph.resolution()?;
        let symbol_data = resolution.symbol(symbol)?;

        Some(symbol_data.name().to_string())
    }

    fn struct_satisfies_fields(&self, struct_symbol: SymbolId) -> Vec<StructFieldInfo> {
        let Some(resolution) = self.graph.resolution() else {
            return Vec::new();
        };

        let Some(member_scope) = resolution.member_scope(struct_symbol) else {
            return Vec::new();
        };

        let Some(scope) = resolution.scope(member_scope) else {
            return Vec::new();
        };

        scope
            .symbols()
            .iter()
            .filter_map(|(name, symbol)| {
                let symbol_data = resolution.symbol(*symbol)?;

                if symbol_data.kind() != SymbolKind::StructField {
                    return None;
                }

                let ty = self.layer.symbol_type(*symbol)?;

                Some(StructFieldInfo {
                    node: symbol_data.declaration(),
                    name: name.to_string(),
                    ty,
                })
            })
            .collect()
    }

    fn constraint_fields(&self, constraint_symbol: SymbolId) -> Vec<ConstraintFieldInfo> {
        let Some(resolution) = self.graph.resolution() else {
            return Vec::new();
        };

        let Some(member_scope) = resolution.member_scope(constraint_symbol) else {
            return Vec::new();
        };

        let Some(scope) = resolution.scope(member_scope) else {
            return Vec::new();
        };

        scope
            .symbols()
            .iter()
            .filter_map(|(name, symbol)| {
                let symbol_data = resolution.symbol(*symbol)?;

                if symbol_data.kind() != SymbolKind::ConstraintField {
                    return None;
                }

                let ty = self.layer.symbol_type(*symbol)?;

                Some(ConstraintFieldInfo {
                    name: name.to_string(),
                    ty,
                })
            })
            .collect()
    }

    fn constraint_functions(&self, constraint_symbol: SymbolId) -> Vec<ConstraintFunctionInfo> {
        let Some(resolution) = self.graph.resolution() else {
            return Vec::new();
        };

        let Some(member_scope) = resolution.member_scope(constraint_symbol) else {
            return Vec::new();
        };

        let Some(scope) = resolution.scope(member_scope) else {
            return Vec::new();
        };

        scope
            .symbols()
            .iter()
            .filter_map(|(name, symbol)| {
                let symbol_data = resolution.symbol(*symbol)?;

                if symbol_data.kind() != SymbolKind::ConstraintFunction {
                    return None;
                }

                let ty = self.layer.symbol_type(*symbol)?;

                Some(ConstraintFunctionInfo {
                    name: name.to_string(),
                    ty,
                })
            })
            .collect()
    }

    fn struct_satisfies_functions(&self, struct_name: &str) -> Vec<StructFunctionInfo> {
        let Some(root) = self.graph.syntax().root() else {
            return Vec::new();
        };

        let mut functions = Vec::new();
        self.collect_anchored_function_items(root, struct_name, &mut functions);

        functions
            .into_iter()
            .filter_map(|function| {
                let (_, function_name) = self.function_anchor_and_name(function)?;
                let symbol = self.function_item_symbol(function, function_name.as_str())?;
                let ty = self.layer.symbol_type(symbol)?;

                Some(StructFunctionInfo {
                    node: function,
                    name: function_name,
                    ty,
                })
            })
            .collect()
    }

    fn collect_anchored_function_items(
        &self,
        node: NodeId,
        struct_name: &str,
        functions: &mut Vec<NodeId>,
    ) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        if syntax_node.kind() == SyntaxNodeKind::FunctionItem {
            if let Some((anchor_name, _)) = self.function_anchor_and_name(node) {
                if anchor_name == struct_name {
                    functions.push(node);
                }
            }

            return;
        }

        for child in syntax_node.children() {
            self.collect_anchored_function_items(*child, struct_name, functions);
        }
    }

    fn function_anchor_and_name(&self, function: NodeId) -> Option<(String, String)> {
        let function_node = self.graph.syntax().node(function)?;
        let children = function_node.children();

        let anchor_index = children.iter().position(|child| {
            self.graph
                .syntax()
                .node(*child)
                .map(|node| node.kind() == SyntaxNodeKind::FunctionAnchor)
                .unwrap_or(false)
        })?;

        let anchor = *children.get(anchor_index)?;

        let name = children
            .iter()
            .skip(anchor_index + 1)
            .copied()
            .find(|child| {
                self.graph
                    .syntax()
                    .node(*child)
                    .map(|node| node.kind() == SyntaxNodeKind::Identifier)
                    .unwrap_or(false)
            })?;

        Some((self.node_text(anchor), self.node_text(name)))
    }

    fn function_item_symbol(&self, function: NodeId, function_name: &str) -> Option<SymbolId> {
        let resolution = self.graph.resolution()?;

        if let Some(symbol) = resolution.declaration_symbol(function) {
            let symbol_data = resolution.symbol(symbol)?;

            if symbol_data.kind() == SymbolKind::Function {
                return Some(symbol);
            }
        }

        let function_node = self.graph.syntax().node(function)?;

        for child in function_node.children() {
            let Some(child_node) = self.graph.syntax().node(*child) else {
                continue;
            };

            if child_node.kind() != SyntaxNodeKind::Identifier {
                continue;
            }

            if self.node_text(*child) != function_name {
                continue;
            }

            let Some(symbol) = resolution.declaration_symbol(*child) else {
                continue;
            };

            let Some(symbol_data) = resolution.symbol(symbol) else {
                continue;
            };

            if symbol_data.kind() == SymbolKind::Function {
                return Some(symbol);
            }
        }

        resolution
            .symbols()
            .iter()
            .find(|symbol| {
                symbol.kind() == SymbolKind::Function
                    && (symbol.declaration() == function
                        || symbol.name() == function_name
                        || symbol
                            .name()
                            .ends_with(format!("::{function_name}").as_str()))
            })
            .map(|symbol| symbol.id())
    }

    fn constraint_application(
        &mut self,
        type_node: NodeId,
    ) -> Result<ConstraintApplication, ConstraintApplicationError> {
        let Some(constraint_symbol) = self.constraint_application_base_symbol(type_node) else {
            return Err(ConstraintApplicationError::InvalidTarget);
        };

        let Some(constraint_name) = self.symbol_name(constraint_symbol) else {
            return Err(ConstraintApplicationError::InvalidTarget);
        };

        let generic_parameters = self.constraint_generic_parameters(constraint_symbol);
        let explicit_arguments = self.constraint_application_argument_types(type_node);

        if generic_parameters.len() != explicit_arguments.len() {
            return Err(ConstraintApplicationError::GenericArgumentCountMismatch {
                constraint_name,
                expected: generic_parameters.len(),
                actual: explicit_arguments.len(),
            });
        }

        let substitution = generic_parameters
            .into_iter()
            .zip(explicit_arguments)
            .collect::<TypeSubstitution>();

        Ok(ConstraintApplication {
            symbol: constraint_symbol,
            substitution,
        })
    }

    fn constraint_application_base_symbol(&self, type_node: NodeId) -> Option<SymbolId> {
        let generic_type = self.find_generic_type_node(type_node);

        let base = if let Some(generic_type) = generic_type {
            let syntax = self.graph.syntax();
            let node = syntax.node(generic_type)?;

            node.children().iter().copied().find(|child| {
                syntax
                    .node(*child)
                    .is_some_and(|child_node| child_node.kind() != SyntaxNodeKind::TypeArgumentList)
            })?
        } else {
            type_node
        };

        self.constraint_symbol_from_base_node(base)
    }

    fn constraint_symbol_from_base_node(&self, base: NodeId) -> Option<SymbolId> {
        let resolution = self.graph.resolution()?;

        if let Some(symbol) = resolution.type_reference_symbol(base) {
            return self.ensure_constraint_symbol(symbol);
        }

        if let Some(symbol) = resolution.type_path_reference_symbol(base) {
            return self.ensure_constraint_symbol(symbol);
        }

        let base_name = self.constraint_base_name(base)?;

        resolution
            .symbols()
            .iter()
            .find(|symbol| symbol.name() == base_name && symbol.kind() == SymbolKind::Constraint)
            .map(|symbol| symbol.id())
    }

    fn ensure_constraint_symbol(&self, symbol: SymbolId) -> Option<SymbolId> {
        let resolution = self.graph.resolution()?;
        let symbol_data = resolution.symbol(symbol)?;

        if symbol_data.kind() != SymbolKind::Constraint {
            return None;
        }

        Some(symbol)
    }

    fn constraint_base_name(&self, base: NodeId) -> Option<String> {
        let syntax = self.graph.syntax();
        let node = syntax.node(base)?;

        match node.kind() {
            SyntaxNodeKind::Identifier => Some(self.node_text(base)),

            SyntaxNodeKind::NamedType | SyntaxNodeKind::Path => {
                let identifier = syntax.first_child_of_kind(base, SyntaxNodeKind::Identifier)?;
                Some(self.node_text(identifier))
            }

            _ => None,
        }
    }

    fn constraint_application_argument_types(&self, type_node: NodeId) -> Vec<TypeId> {
        let Some(generic_type) = self.find_generic_type_node(type_node) else {
            return Vec::new();
        };

        let Some(ty) = self.layer.node_type(generic_type) else {
            return Vec::new();
        };

        match self.layer.table().kind(ty) {
            Some(TypeKind::GenericInstance { arguments, .. }) => arguments.clone(),
            _ => Vec::new(),
        }
    }

    fn find_generic_type_node(&self, node: NodeId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        if syntax_node.kind() == SyntaxNodeKind::GenericType {
            return Some(node);
        }

        for child in syntax_node.children() {
            if let Some(found) = self.find_generic_type_node(*child) {
                return Some(found);
            }
        }

        None
    }

    pub(super) fn constraint_generic_parameters(
        &self,
        constraint_symbol: SymbolId,
    ) -> Vec<SymbolId> {
        if let Some(parameters) = self.builtin_constraint_generic_parameters(constraint_symbol) {
            return parameters;
        }

        let Some(constraint_item) = self.constraint_item_for_symbol(constraint_symbol) else {
            return Vec::new();
        };

        self.declaration_symbols_in_node(constraint_item, &[SymbolKind::GenericParameter])
    }

    fn constraint_item_for_symbol(&self, constraint_symbol: SymbolId) -> Option<NodeId> {
        let constraint_name = self.symbol_name(constraint_symbol)?;
        let root = self.graph.syntax().root()?;

        self.find_constraint_item_by_name(root, constraint_name.as_str())
    }

    fn find_constraint_item_by_name(&self, node: NodeId, constraint_name: &str) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        if syntax_node.kind() == SyntaxNodeKind::ConstraintItem {
            let name = self
                .graph
                .syntax()
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)?;

            if self.node_text(name) == constraint_name {
                return Some(node);
            }

            return None;
        }

        for child in syntax_node.children() {
            if let Some(found) = self.find_constraint_item_by_name(*child, constraint_name) {
                return Some(found);
            }
        }

        None
    }

    fn substitute_type(&mut self, ty: TypeId, substitution: &TypeSubstitution) -> TypeId {
        let ty = self.resolve_alias_type(ty);

        match self.layer.table().kind(ty).cloned() {
            Some(TypeKind::GenericParameter { symbol }) => {
                substitution.get(&symbol).copied().unwrap_or(ty)
            }

            Some(TypeKind::Array { element }) => {
                let element = self.substitute_type(element, substitution);
                self.layer.table_mut().intern_array(element)
            }

            Some(TypeKind::FixedArray { element, size }) => {
                let element = self.substitute_type(element, substitution);
                self.layer.table_mut().intern_fixed_array(element, size)
            }

            Some(TypeKind::Union { members }) => {
                let members = members
                    .into_iter()
                    .map(|member| self.substitute_type(member, substitution))
                    .collect::<Vec<_>>();

                self.layer.table_mut().intern_union(members)
            }

            Some(TypeKind::Tuple { elements }) => {
                let elements = elements
                    .into_iter()
                    .map(|element| self.substitute_type(element, substitution))
                    .collect::<Vec<_>>();

                self.layer.table_mut().intern_tuple(elements)
            }

            Some(TypeKind::Function(function)) => {
                let parameters = function
                    .parameters()
                    .iter()
                    .map(|parameter| {
                        let ty = self.substitute_type(parameter.ty(), substitution);

                        if parameter.is_rest() {
                            return FunctionParameterType::rest(ty);
                        }

                        if parameter.has_default() {
                            return FunctionParameterType::with_default(ty);
                        }

                        FunctionParameterType::new(ty)
                    })
                    .collect::<Vec<_>>();

                let return_type = self.substitute_type(function.return_type(), substitution);

                self.layer
                    .table_mut()
                    .intern_function(parameters, return_type)
            }

            Some(TypeKind::Error) => ty,

            _ => ty,
        }
    }

    pub(super) fn satisfied_constraint_application(
        &mut self,
        ty: TypeId,
        constraint_name: &str,
    ) -> Option<ConstraintApplication> {
        let ty = self.resolve_alias_type(ty);

        let symbol = match self.layer.table().kind(ty).cloned()? {
            TypeKind::Named { symbol } => symbol,
            _ => return None,
        };

        let resolution = self.graph.resolution()?;
        let symbol_data = resolution.symbol(symbol)?;

        if symbol_data.kind() != SymbolKind::Struct {
            return None;
        }

        let struct_item = self.type_item_for_symbol(symbol)?;
        let satisfies = self
            .graph
            .syntax()
            .first_child_of_kind(struct_item, SyntaxNodeKind::SatisfiesClause)?;

        let constraints = self
            .graph
            .syntax()
            .node(satisfies)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        for constraint_type in constraints {
            let Ok(application) = self.constraint_application(constraint_type) else {
                continue;
            };

            let Some(name) = self.symbol_name(application.symbol) else {
                continue;
            };

            if name == constraint_name {
                return Some(application);
            }
        }

        None
    }

    fn type_item_for_symbol(&self, symbol: SymbolId) -> Option<NodeId> {
        let resolution = self.graph.resolution()?;
        let member_scope = resolution.member_scope(symbol)?;
        let scope = resolution.scope(member_scope)?;
        scope.owner()
    }

    fn check_generic_parameter_constraint(&mut self, constraint: NodeId) {
        let Some(constraint_type) = self.first_constraint_type_child(constraint) else {
            return;
        };

        let target_name = self.node_text(constraint_type);

        match self.constraint_application(constraint_type) {
            Ok(_) => {}
            Err(ConstraintApplicationError::InvalidTarget) => {
                self.report_invalid_satisfies_target(constraint_type, target_name.as_str());
            }
            Err(ConstraintApplicationError::GenericArgumentCountMismatch {
                constraint_name,
                expected,
                actual,
            }) => {
                self.report_constraint_generic_argument_count_mismatch(
                    constraint_type,
                    constraint_name.as_str(),
                    expected,
                    actual,
                );
            }
        }
    }

    fn first_constraint_type_child(&self, node: NodeId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        for child in syntax_node.children() {
            let Some(child_node) = self.graph.syntax().node(*child) else {
                continue;
            };

            if matches!(
                child_node.kind(),
                SyntaxNodeKind::NamedType | SyntaxNodeKind::Path | SyntaxNodeKind::GenericType
            ) {
                return Some(*child);
            }

            if matches!(
                child_node.kind(),
                SyntaxNodeKind::BasicConstraint | SyntaxNodeKind::GenericParameterConstraint
            ) {
                if let Some(found) = self.first_constraint_type_child(*child) {
                    return Some(found);
                }
            }
        }

        None
    }
}
