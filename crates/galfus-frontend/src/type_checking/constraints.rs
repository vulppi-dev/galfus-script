use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{SymbolKind, SyntaxNodeKind};

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

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn check_constraint_satisfies(&mut self, node: NodeId) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        if syntax_node.kind() == SyntaxNodeKind::StructItem {
            self.check_struct_satisfies(node);
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
            );
        }
    }

    fn check_single_satisfies_constraint(
        &mut self,
        struct_item: NodeId,
        constraint_type: NodeId,
        struct_name: &str,
        struct_fields: &[StructFieldInfo],
    ) {
        let constraint_name = self.node_text(constraint_type);

        let Some(constraint_symbol) = self.constraint_symbol_from_type_node(constraint_type) else {
            self.report_invalid_satisfies_target(constraint_type, constraint_name.as_str());
            return;
        };

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

            if self.is_assignable(constraint_field.ty, struct_field.ty) {
                continue;
            }

            self.report_constraint_field_type_mismatch(
                struct_field.node,
                struct_name,
                constraint_name.as_str(),
                constraint_field.name.as_str(),
                constraint_field.ty,
                struct_field.ty,
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

    fn constraint_symbol_from_type_node(&self, type_node: NodeId) -> Option<SymbolId> {
        let ty = self.layer.node_type(type_node)?;

        let ty = self.resolve_alias_type(ty);

        let symbol = match self.layer.table().kind(ty) {
            Some(crate::TypeKind::Named { symbol }) => *symbol,
            _ => return None,
        };

        let resolution = self.graph.resolution()?;
        let symbol_data = resolution.symbol(symbol)?;

        if symbol_data.kind() != SymbolKind::Constraint {
            return None;
        }

        Some(symbol)
    }

    fn symbol_name(&self, symbol: SymbolId) -> Option<String> {
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
}
