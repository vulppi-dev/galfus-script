#[cfg(test)]
mod tests;

use crate::{
    FunctionParameterType, ModuleGraph, PrimitiveType, SymbolKind, SyntaxNodeKind,
    TypeDiagnosticCode, TypeKind, TypeLayer, lower_types,
};
use galfus_core::{Diagnostic, DiagnosticBag, NodeId, SourceFile, SymbolId, TypeId};

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
        self.bind_named_type_definition_symbols();

        let Some(root) = self.graph.syntax().root() else {
            return;
        };

        self.check_node(root);
        self.check_initializer_types(root);
        self.check_return_types(root, None);
    }

    fn check_node(&mut self, node: NodeId) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        match syntax_node.kind() {
            SyntaxNodeKind::FunctionItem => {
                self.bind_function_item_type(node);
            }

            SyntaxNodeKind::ConstraintFunctionSignature => {
                self.bind_constraint_function_signature_type(node);
            }

            SyntaxNodeKind::Parameter | SyntaxNodeKind::RestParameter => {
                self.bind_direct_declaration_type(
                    node,
                    &[SymbolKind::Parameter, SymbolKind::RestParameter],
                );
            }

            SyntaxNodeKind::StructField | SyntaxNodeKind::WeakStructField => {
                self.bind_direct_declaration_type(node, &[SymbolKind::StructField]);
            }

            SyntaxNodeKind::ConstraintField => {
                self.bind_direct_declaration_type(node, &[SymbolKind::ConstraintField]);
            }

            SyntaxNodeKind::VarItem
            | SyntaxNodeKind::ConstItem
            | SyntaxNodeKind::VarStatement
            | SyntaxNodeKind::ConstStatement => {
                self.bind_binding_declaration_type(node);
            }

            SyntaxNodeKind::TypeAliasItem => {
                self.bind_type_alias_type(node);
            }

            SyntaxNodeKind::GenericParameter => {
                self.bind_generic_parameter_type(node);
            }

            _ => {}
        }

        for child in syntax_node.children() {
            self.check_node(*child);
        }
    }

    fn bind_builtin_symbol_types(&mut self) {
        let Some(resolution) = self.graph.resolution() else {
            return;
        };

        for symbol in resolution.symbols() {
            if symbol.kind() != SymbolKind::BuiltinType {
                continue;
            }

            let Some(primitive) = primitive_type_by_name(symbol.name()) else {
                continue;
            };

            let ty = self.layer.table().primitive(primitive);
            self.layer.bind_symbol_type(symbol.id(), ty);
        }
    }

    fn bind_named_type_definition_symbols(&mut self) {
        let Some(resolution) = self.graph.resolution() else {
            return;
        };

        let symbols = resolution.symbols().to_vec();

        for symbol in symbols {
            match symbol.kind() {
                SymbolKind::Struct
                | SymbolKind::Enum
                | SymbolKind::Choice
                | SymbolKind::Constraint => {
                    let ty = self.layer.table_mut().intern_named(symbol.id());
                    self.layer.bind_symbol_type(symbol.id(), ty);
                }

                _ => {}
            }
        }
    }

    fn bind_function_item_type(&mut self, node: NodeId) {
        let Some(symbol) = self.direct_identifier_symbol(node, SymbolKind::Function) else {
            return;
        };

        let Some(ty) = self.lower_function_signature_type(node) else {
            return;
        };

        self.layer.bind_symbol_type(symbol, ty);
    }

    fn bind_constraint_function_signature_type(&mut self, node: NodeId) {
        let Some(symbol) = self.direct_identifier_symbol(node, SymbolKind::ConstraintFunction)
        else {
            return;
        };

        let Some(ty) = self.lower_function_signature_type(node) else {
            return;
        };

        self.layer.bind_symbol_type(symbol, ty);
    }

    fn lower_function_signature_type(&mut self, node: NodeId) -> Option<TypeId> {
        let parameters_node = self
            .graph
            .syntax()
            .first_child_of_kind(node, SyntaxNodeKind::ParameterList)?;

        let return_type_node = self.last_direct_type_child(node)?;

        let parameters = self
            .graph
            .syntax()
            .node(parameters_node)?
            .children()
            .iter()
            .filter_map(|parameter| self.lower_function_parameter_type(*parameter))
            .collect::<Vec<_>>();

        let return_type = self.layer.node_type(return_type_node)?;

        Some(
            self.layer
                .table_mut()
                .intern_function(parameters, return_type),
        )
    }

    fn lower_function_parameter_type(
        &mut self,
        parameter: NodeId,
    ) -> Option<FunctionParameterType> {
        let parameter_node = self.graph.syntax().node(parameter)?;

        match parameter_node.kind() {
            SyntaxNodeKind::Parameter | SyntaxNodeKind::RestParameter => {}

            _ => return None,
        }

        let type_node = self.first_type_child(parameter)?;
        let ty = self.layer.node_type(type_node)?;

        let has_default = self
            .graph
            .syntax()
            .first_child_of_kind(parameter, SyntaxNodeKind::ParameterDefault)
            .is_some();

        if parameter_node.kind() == SyntaxNodeKind::RestParameter {
            return Some(FunctionParameterType::rest(ty));
        }

        if has_default {
            return Some(FunctionParameterType::with_default(ty));
        }

        Some(FunctionParameterType::new(ty))
    }

    fn bind_direct_declaration_type(&mut self, node: NodeId, kinds: &[SymbolKind]) {
        let Some(symbol) = self.direct_identifier_symbol_any(node, kinds) else {
            return;
        };

        let Some(type_node) = self.first_type_child(node) else {
            return;
        };

        let Some(ty) = self.layer.node_type(type_node) else {
            return;
        };

        self.layer.bind_symbol_type(symbol, ty);
    }

    fn bind_binding_declaration_type(&mut self, node: NodeId) {
        let Some(type_annotation) = self
            .graph
            .syntax()
            .first_child_of_kind(node, SyntaxNodeKind::TypeAnnotation)
        else {
            return;
        };

        let Some(type_node) = self.first_type_child(type_annotation) else {
            return;
        };

        let Some(ty) = self.layer.node_type(type_node) else {
            return;
        };

        let symbols = self.declaration_symbols_in_node(
            node,
            &[
                SymbolKind::Var,
                SymbolKind::Const,
                SymbolKind::PatternBinding,
                SymbolKind::TypePatternBinding,
            ],
        );

        for symbol in symbols {
            self.layer.bind_symbol_type(symbol, ty);
        }
    }

    fn bind_type_alias_type(&mut self, node: NodeId) {
        let Some(symbol) = self.direct_identifier_symbol(node, SymbolKind::TypeAlias) else {
            return;
        };

        let Some(type_node) = self.last_direct_type_child(node) else {
            return;
        };

        let Some(ty) = self.layer.node_type(type_node) else {
            return;
        };

        self.layer.bind_symbol_type(symbol, ty);
    }

    fn bind_generic_parameter_type(&mut self, node: NodeId) {
        let Some(symbol) = self.direct_identifier_symbol(node, SymbolKind::GenericParameter) else {
            return;
        };

        let ty = self.layer.table_mut().intern_generic_parameter(symbol);
        self.layer.bind_symbol_type(symbol, ty);
    }

    fn direct_identifier_symbol(&self, node: NodeId, kind: SymbolKind) -> Option<SymbolId> {
        self.direct_identifier_symbol_any(node, &[kind])
    }

    fn direct_identifier_symbol_any(&self, node: NodeId, kinds: &[SymbolKind]) -> Option<SymbolId> {
        let resolution = self.graph.resolution()?;
        let syntax_node = self.graph.syntax().node(node)?;

        for child in syntax_node.children() {
            let child_node = self.graph.syntax().node(*child)?;

            if child_node.kind() != SyntaxNodeKind::Identifier {
                continue;
            }

            let Some(symbol) = resolution.declaration_symbol(*child) else {
                continue;
            };

            let Some(symbol_data) = resolution.symbol(symbol) else {
                continue;
            };

            if kinds.contains(&symbol_data.kind()) {
                return Some(symbol);
            }
        }

        None
    }

    fn declaration_symbols_in_node(&self, node: NodeId, kinds: &[SymbolKind]) -> Vec<SymbolId> {
        let mut symbols = Vec::new();
        self.collect_declaration_symbols_in_node(node, kinds, &mut symbols);
        symbols
    }

    fn collect_declaration_symbols_in_node(
        &self,
        node: NodeId,
        kinds: &[SymbolKind],
        symbols: &mut Vec<SymbolId>,
    ) {
        let Some(resolution) = self.graph.resolution() else {
            return;
        };

        if let Some(symbol) = resolution.declaration_symbol(node) {
            if let Some(symbol_data) = resolution.symbol(symbol) {
                if kinds.contains(&symbol_data.kind()) {
                    symbols.push(symbol);
                }
            }
        }

        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        for child in syntax_node.children() {
            self.collect_declaration_symbols_in_node(*child, kinds, symbols);
        }
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

    fn last_direct_type_child(&self, node: NodeId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        syntax_node
            .children()
            .iter()
            .rev()
            .copied()
            .find(|child| self.is_type_node(*child))
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

    fn check_initializer_types(&mut self, node: NodeId) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        match syntax_node.kind() {
            SyntaxNodeKind::VarItem
            | SyntaxNodeKind::ConstItem
            | SyntaxNodeKind::VarStatement
            | SyntaxNodeKind::ConstStatement => {
                self.check_binding_initializer_type(node);
            }

            _ => {}
        }

        for child in syntax_node.children() {
            self.check_initializer_types(*child);
        }
    }

    fn check_binding_initializer_type(&mut self, node: NodeId) {
        let Some(type_annotation) = self
            .graph
            .syntax()
            .first_child_of_kind(node, SyntaxNodeKind::TypeAnnotation)
        else {
            return;
        };

        let Some(type_node) = self.first_type_child(type_annotation) else {
            return;
        };

        let Some(expected) = self.layer.node_type(type_node) else {
            return;
        };

        let Some(initializer) = self
            .graph
            .syntax()
            .first_child_of_kind(node, SyntaxNodeKind::Initializer)
        else {
            return;
        };

        let Some(expression) = self.graph.syntax().child(initializer, 0) else {
            return;
        };

        let Some(actual) = self.infer_expression_type(expression) else {
            return;
        };

        if self.is_assignable(expected, actual) {
            return;
        }

        self.report_type_mismatch(expression, expected, actual);
    }

    fn infer_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        if let Some(existing) = self.layer.node_type(node) {
            return Some(existing);
        }

        let syntax_node = self.graph.syntax().node(node)?;

        let ty = match syntax_node.kind() {
            SyntaxNodeKind::IntegerLiteral => {
                Some(self.layer.table().primitive(PrimitiveType::Int32))
            }

            SyntaxNodeKind::FloatLiteral => {
                Some(self.layer.table().primitive(PrimitiveType::Float64))
            }

            SyntaxNodeKind::BoolLiteral => Some(self.layer.table().primitive(PrimitiveType::Bool)),

            SyntaxNodeKind::NullLiteral => Some(self.layer.table().primitive(PrimitiveType::Null)),

            SyntaxNodeKind::GroupedExpression => {
                let inner = self.graph.syntax().child(node, 0)?;
                self.infer_expression_type(inner)
            }

            SyntaxNodeKind::NameExpression => self.infer_name_expression_type(node),

            SyntaxNodeKind::CallExpression => self.infer_call_expression_type(node),

            SyntaxNodeKind::TupleExpression => self.infer_tuple_expression_type(node),

            SyntaxNodeKind::CastExpression => self.infer_cast_expression_type(node),

            SyntaxNodeKind::CopyExpression => {
                let value = self.graph.syntax().child(node, 0)?;
                self.infer_expression_type(value)
            }

            _ => None,
        }?;

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    fn infer_name_expression_type(&self, node: NodeId) -> Option<TypeId> {
        let resolution = self.graph.resolution()?;

        let symbol = resolution.reference_symbol(node).or_else(|| {
            let identifier = self
                .graph
                .syntax()
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)?;

            resolution.reference_symbol(identifier)
        })?;

        self.layer.symbol_type(symbol)
    }

    fn infer_tuple_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let elements = self
            .graph
            .syntax()
            .node(node)?
            .children()
            .to_vec()
            .into_iter()
            .map(|child| self.infer_expression_type(child))
            .collect::<Option<Vec<_>>>()?;

        Some(self.layer.table_mut().intern_tuple(elements))
    }

    fn infer_cast_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let type_node = self.first_type_child(node)?;
        self.layer.node_type(type_node)
    }

    fn is_assignable(&self, expected: TypeId, actual: TypeId) -> bool {
        if expected == actual {
            return true;
        }

        let expected_kind = self.layer.table().kind(expected).cloned();
        let actual_kind = self.layer.table().kind(actual).cloned();

        if matches!(expected_kind, Some(TypeKind::Error)) {
            return true;
        }

        if matches!(actual_kind, Some(TypeKind::Error)) {
            return true;
        }

        match expected_kind {
            Some(TypeKind::Union { members }) => members.contains(&actual),
            _ => false,
        }
    }

    fn report_type_mismatch(&mut self, expression: NodeId, expected: TypeId, actual: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(expression)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let expected = self.layer.table().describe(expected);
        let actual = self.layer.table().describe(actual);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::TypeMismatch,
            format!("expected `{expected}`, got `{actual}`"),
            span,
        ));
    }

    fn check_return_types(&mut self, node: NodeId, current_return_type: Option<TypeId>) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        match syntax_node.kind() {
            SyntaxNodeKind::FunctionItem => {
                let function_return_type = self
                    .last_direct_type_child(node)
                    .and_then(|return_type| self.layer.node_type(return_type));

                for child in syntax_node.children() {
                    self.check_return_types(*child, function_return_type);
                }

                return;
            }

            SyntaxNodeKind::ReturnStatement => {
                self.check_return_statement_type(node, current_return_type);
            }

            _ => {}
        }

        for child in syntax_node.children() {
            self.check_return_types(*child, current_return_type);
        }
    }

    fn check_return_statement_type(&mut self, return_statement: NodeId, expected: Option<TypeId>) {
        let Some(expected) = expected else {
            return;
        };

        let actual = match self.graph.syntax().child(return_statement, 0) {
            Some(expression) => match self.infer_expression_type(expression) {
                Some(actual) => actual,
                None => return,
            },

            None => self.layer.table().primitive(PrimitiveType::Null),
        };

        if self.is_assignable(expected, actual) {
            return;
        }

        let diagnostic_node = self
            .graph
            .syntax()
            .child(return_statement, 0)
            .unwrap_or(return_statement);

        self.report_type_mismatch(diagnostic_node, expected, actual);
    }

    fn infer_call_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let target = self.graph.syntax().child(node, 0)?;
        let arguments = self.graph.syntax().child(node, 1)?;

        let target_type = self.infer_expression_type(target)?;

        let function = match self.layer.table().kind(target_type).cloned() {
            Some(TypeKind::Function(function)) => function,
            Some(TypeKind::Error) => return Some(self.layer.table_mut().error()),
            _ => {
                self.report_not_callable(target, target_type);
                return Some(self.layer.table_mut().error());
            }
        };

        let argument_nodes = self.call_argument_nodes(arguments);

        self.check_call_argument_count(node, &function, argument_nodes.len());

        self.check_call_argument_types(argument_nodes.as_slice(), &function);

        Some(function.return_type())
    }

    fn call_argument_nodes(&self, arguments: NodeId) -> Vec<NodeId> {
        let Some(arguments_node) = self.graph.syntax().node(arguments) else {
            return Vec::new();
        };

        arguments_node
            .children()
            .iter()
            .filter_map(|child| self.call_argument_expression(*child))
            .collect()
    }

    fn call_argument_expression(&self, node: NodeId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        match syntax_node.kind() {
            SyntaxNodeKind::SpreadArgument => self.graph.syntax().child(node, 0),
            _ => Some(node),
        }
    }

    fn check_call_argument_count(
        &mut self,
        call: NodeId,
        function: &crate::FunctionType,
        argument_count: usize,
    ) {
        let parameters = function.parameters();

        let required_count = parameters
            .iter()
            .filter(|parameter| !parameter.has_default() && !parameter.is_rest())
            .count();

        let has_rest = parameters.iter().any(|parameter| parameter.is_rest());

        let max_count = if has_rest {
            None
        } else {
            Some(parameters.len())
        };

        let too_few = argument_count < required_count;
        let too_many = max_count.is_some_and(|max_count| argument_count > max_count);

        if !too_few && !too_many {
            return;
        }

        let expected = match max_count {
            Some(max_count) if required_count == max_count => required_count.to_string(),
            Some(max_count) => {
                format!("{required_count}..{max_count}")
            }
            None => {
                format!("{required_count}+")
            }
        };

        let span = self
            .graph
            .syntax()
            .node(call)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::ArgumentCountMismatch,
            format!("expected {expected} arguments, got {argument_count}"),
            span,
        ));
    }

    fn check_call_argument_types(
        &mut self,
        argument_nodes: &[NodeId],
        function: &crate::FunctionType,
    ) {
        for (index, argument) in argument_nodes.iter().copied().enumerate() {
            let Some(expected) = self.call_parameter_type(function, index) else {
                continue;
            };

            let Some(actual) = self.infer_expression_type(argument) else {
                continue;
            };

            if self.is_assignable(expected, actual) {
                continue;
            }

            self.report_type_mismatch(argument, expected, actual);
        }
    }

    fn call_parameter_type(
        &self,
        function: &crate::FunctionType,
        argument_index: usize,
    ) -> Option<TypeId> {
        let parameters = function.parameters();

        if let Some(parameter) = parameters.get(argument_index) {
            return Some(parameter.ty());
        }

        let rest = parameters.iter().find(|parameter| parameter.is_rest())?;

        Some(rest.ty())
    }

    fn report_not_callable(&mut self, target: NodeId, target_type: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(target)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let actual = self.layer.table().describe(target_type);

        self.diagnostics.push(Diagnostic::error_with_message(
            TypeDiagnosticCode::NotCallable,
            format!("type `{actual}` is not callable"),
            span,
        ));
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

pub fn check_declaration_types(source: &SourceFile, graph: &ModuleGraph) -> TypeCheckResult {
    let lowering = lower_types(source, graph);

    let mut checker = DeclarationTypeChecker::new(source, graph, lowering.into_layer());
    checker.check();
    checker.into_result()
}
