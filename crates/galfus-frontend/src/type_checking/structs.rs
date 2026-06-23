use std::collections::HashSet;

use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{SymbolKind, SyntaxNodeKind};

use super::DeclarationTypeChecker;

#[derive(Debug, Clone)]
struct StructFieldInfo {
    name: String,
    ty: TypeId,
    has_default: bool,
}

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_struct_literal_type(&mut self, node: NodeId) -> Option<TypeId> {
        let target = self.graph.syntax().child(node, 0)?;
        let fields = self.graph.syntax().child(node, 1)?;
        let target_name = self.node_text(target);

        let Some((struct_symbol, target_type, struct_name)) = self.struct_literal_target(target)
        else {
            self.report_invalid_struct_literal_target(target, target_name.as_str());

            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        };

        let ty =
            self.check_struct_literal_fields(node, fields, struct_symbol, target_type, struct_name);

        self.layer.bind_node_type(node, ty);

        Some(ty)
    }

    pub(super) fn check_struct_literal_fields(
        &mut self,
        literal: NodeId,
        fields: NodeId,
        struct_symbol: SymbolId,
        target_type: TypeId,
        struct_name: String,
    ) -> TypeId {
        let expected_fields = self.struct_fields(struct_symbol);
        let mut provided = HashSet::new();
        let mut has_error = false;

        let field_nodes = self
            .graph
            .syntax()
            .node(fields)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();

        for field in field_nodes {
            let Some(field_name) = self.struct_literal_field_name(field) else {
                continue;
            };

            if !provided.insert(field_name.clone()) {
                self.report_duplicate_struct_field(field, field_name.as_str());
                has_error = true;
                continue;
            }

            let Some(expected_field) = expected_fields
                .iter()
                .find(|candidate| candidate.name == field_name)
            else {
                self.report_unknown_struct_field(field, field_name.as_str(), struct_name.as_str());
                has_error = true;
                continue;
            };

            let Some((value_node, actual)) =
                self.struct_literal_field_value_type(field, field_name.as_str())
            else {
                continue;
            };

            if self.is_assignable(expected_field.ty, actual) {
                continue;
            }

            self.report_type_mismatch(value_node, expected_field.ty, actual);
            has_error = true;
        }

        for expected_field in expected_fields.iter() {
            if provided.contains(expected_field.name.as_str()) {
                continue;
            }

            if expected_field.has_default {
                continue;
            }

            self.report_missing_struct_field(
                literal,
                expected_field.name.as_str(),
                struct_name.as_str(),
            );

            has_error = true;
        }

        if has_error {
            self.layer.table_mut().error()
        } else {
            target_type
        }
    }

    fn struct_literal_target(&mut self, target: NodeId) -> Option<(SymbolId, TypeId, String)> {
        let target_name = self.node_text(target);

        let (symbol, struct_name) = {
            let resolution = self.graph.resolution()?;

            let symbol = resolution.reference_symbol(target).or_else(|| {
                resolution
                    .symbols()
                    .iter()
                    .find(|symbol| {
                        symbol.name() == target_name && symbol.kind() == SymbolKind::Struct
                    })
                    .map(|symbol| symbol.id())
            })?;

            let symbol_data = resolution.symbol(symbol)?;

            if symbol_data.kind() != SymbolKind::Struct {
                return None;
            }

            (symbol, symbol_data.name().to_string())
        };

        let ty = self
            .layer
            .symbol_type(symbol)
            .unwrap_or_else(|| self.layer.table_mut().intern_named(symbol));

        Some((symbol, ty, struct_name))
    }

    fn struct_fields(&self, struct_symbol: SymbolId) -> Vec<StructFieldInfo> {
        let Some(resolution) = self.graph.resolution() else {
            return Vec::new();
        };

        let Some(struct_symbol_data) = resolution.symbol(struct_symbol) else {
            return Vec::new();
        };

        let struct_name = struct_symbol_data.name().to_string();

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
                    name: name.to_string(),
                    ty,
                    has_default: self.struct_field_has_default(
                        struct_name.as_str(),
                        name.as_str(),
                        symbol_data.declaration(),
                    ),
                })
            })
            .collect()
    }

    fn struct_field_has_default(
        &self,
        struct_name: &str,
        field_name: &str,
        declaration: NodeId,
    ) -> bool {
        if self.node_contains_kind(declaration, SyntaxNodeKind::StructFieldDefault) {
            return true;
        }

        let Some(field) = self.struct_field_node_by_name(struct_name, field_name) else {
            return false;
        };

        self.node_contains_kind(field, SyntaxNodeKind::StructFieldDefault)
    }

    fn struct_field_node_by_name(&self, struct_name: &str, field_name: &str) -> Option<NodeId> {
        let root = self.graph.syntax().root()?;
        let struct_item = self.struct_item_node_by_name(root, struct_name)?;

        self.find_struct_field_node_by_name(struct_item, field_name)
    }

    fn struct_item_node_by_name(&self, node: NodeId, struct_name: &str) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        if syntax_node.kind() == SyntaxNodeKind::StructItem {
            let identifier = self
                .graph
                .syntax()
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)?;

            if self.node_text(identifier) == struct_name {
                return Some(node);
            }
        }

        for child in syntax_node.children() {
            if let Some(found) = self.struct_item_node_by_name(*child, struct_name) {
                return Some(found);
            }
        }

        None
    }

    fn find_struct_field_node_by_name(&self, node: NodeId, field_name: &str) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        if syntax_node.kind() == SyntaxNodeKind::StructField {
            let identifier = self
                .graph
                .syntax()
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)?;

            if self.node_text(identifier) == field_name {
                return Some(node);
            }
        }

        for child in syntax_node.children() {
            if let Some(found) = self.find_struct_field_node_by_name(*child, field_name) {
                return Some(found);
            }
        }

        None
    }

    fn node_contains_kind(&self, node: NodeId, kind: SyntaxNodeKind) -> bool {
        if self
            .graph
            .syntax()
            .node(node)
            .map(|node| node.kind() == kind)
            .unwrap_or(false)
        {
            return true;
        }

        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return false;
        };

        for child in syntax_node.children() {
            if self.node_contains_kind(*child, kind) {
                return true;
            }
        }

        false
    }

    fn struct_literal_field_name(&self, field: NodeId) -> Option<String> {
        let field_node = self.graph.syntax().node(field)?;

        match field_node.kind() {
            SyntaxNodeKind::StructLiteralField | SyntaxNodeKind::StructLiteralFieldShorthand => {
                let identifier = self
                    .graph
                    .syntax()
                    .first_child_of_kind(field, SyntaxNodeKind::Identifier)?;

                Some(self.node_text(identifier))
            }

            _ => None,
        }
    }

    fn struct_literal_field_value_type(
        &mut self,
        field: NodeId,
        _field_name: &str,
    ) -> Option<(NodeId, TypeId)> {
        let field_node = self.graph.syntax().node(field)?;

        match field_node.kind() {
            SyntaxNodeKind::StructLiteralField => {
                let value = *field_node.children().get(1)?;
                let ty = self.infer_expression_type(value)?;

                Some((value, ty))
            }

            SyntaxNodeKind::StructLiteralFieldShorthand => {
                let identifier = *field_node.children().first()?;
                let ty = self.infer_identifier_reference_type(identifier)?;

                self.layer.bind_node_type(field, ty);

                Some((identifier, ty))
            }

            _ => None,
        }
    }

    fn infer_identifier_reference_type(&self, identifier: NodeId) -> Option<TypeId> {
        let resolution = self.graph.resolution()?;
        let symbol = resolution.reference_symbol(identifier)?;

        self.layer.symbol_type(symbol)
    }
}
