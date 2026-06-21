use galfus_core::{NodeId, SymbolId};

use crate::{SymbolKind, SyntaxNodeKind};

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn direct_identifier_symbol(
        &self,
        node: NodeId,
        kind: SymbolKind,
    ) -> Option<SymbolId> {
        self.direct_identifier_symbol_any(node, &[kind])
    }

    pub(super) fn direct_identifier_symbol_any(
        &self,
        node: NodeId,
        kinds: &[SymbolKind],
    ) -> Option<SymbolId> {
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

    pub(super) fn declaration_symbols_in_node(
        &self,
        node: NodeId,
        kinds: &[SymbolKind],
    ) -> Vec<SymbolId> {
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

    pub(super) fn first_type_child(&self, node: NodeId) -> Option<NodeId> {
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

    pub(super) fn last_direct_type_child(&self, node: NodeId) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        syntax_node
            .children()
            .iter()
            .rev()
            .copied()
            .find(|child| self.is_type_node(*child))
    }

    pub(super) fn is_type_node(&self, node: NodeId) -> bool {
        self.graph
            .syntax()
            .node(node)
            .map(|node| self.is_type_node_kind(node.kind()))
            .unwrap_or(false)
    }

    pub(super) fn is_type_node_kind(&self, kind: SyntaxNodeKind) -> bool {
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

    pub(super) fn node_text(&self, node: NodeId) -> String {
        let Some(node) = self.graph.syntax().node(node) else {
            return String::new();
        };

        self.source.slice(node.span()).unwrap_or("").to_string()
    }
}
