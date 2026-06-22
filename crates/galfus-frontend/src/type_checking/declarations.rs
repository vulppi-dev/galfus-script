use galfus_core::{NodeId, TypeId};

use crate::{FunctionParameterType, SymbolKind, SyntaxNodeKind};

use super::{DeclarationTypeChecker, primitive_type_by_name};

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn bind_builtin_symbol_types(&mut self) {
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

    pub(super) fn bind_named_type_definition_symbols(&mut self) {
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

    pub(super) fn check_node(&mut self, node: NodeId) {
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
        let Some(type_node) = self.first_type_child(node) else {
            return;
        };

        let Some(ty) = self.layer.node_type(type_node) else {
            return;
        };

        let symbols = self.declaration_symbols_in_node(node, &[SymbolKind::TypeAlias]);

        for symbol in symbols {
            self.layer.bind_symbol_type(symbol, ty);
        }
    }

    fn bind_generic_parameter_type(&mut self, node: NodeId) {
        let Some(symbol) = self.direct_identifier_symbol(node, SymbolKind::GenericParameter) else {
            return;
        };

        let ty = self.layer.table_mut().intern_generic_parameter(symbol);
        self.layer.bind_symbol_type(symbol, ty);
    }
}
