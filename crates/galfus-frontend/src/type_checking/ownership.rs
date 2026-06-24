use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{PrimitiveType, SymbolKind, SyntaxNodeKind, TypeDiagnosticCode, TypeKind};

use super::{DeclarationTypeChecker, WeakFieldMetadata};

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn check_ownership_metadata(&mut self, root: NodeId) {
        self.collect_ownership_metadata(root, None);
    }

    fn collect_ownership_metadata(&mut self, node: NodeId, owner_struct: Option<SymbolId>) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        let owner_struct = if syntax_node.kind() == SyntaxNodeKind::StructItem {
            self.direct_identifier_symbol(node, SymbolKind::Struct)
                .or(owner_struct)
        } else {
            owner_struct
        };

        if syntax_node.kind() == SyntaxNodeKind::WeakStructField {
            self.check_weak_struct_field(node, owner_struct);
        }

        let children = syntax_node.children().to_vec();

        for child in children {
            self.collect_ownership_metadata(child, owner_struct);
        }
    }

    fn check_weak_struct_field(&mut self, field: NodeId, owner_struct: Option<SymbolId>) {
        let Some(owner_struct) = owner_struct else {
            return;
        };

        let Some(field_symbol) = self.direct_identifier_symbol(field, SymbolKind::StructField)
        else {
            return;
        };

        let Some(field_type) = self.layer.symbol_type(field_symbol) else {
            return;
        };

        if !self.is_weak_field_nullable_type(field_type) {
            self.report_invalid_weak_field_type(field, field_type);
        }

        self.ownership_metadata
            .weak_fields
            .push(WeakFieldMetadata::new(
                owner_struct,
                field_symbol,
                field,
                field_type,
            ));
    }

    fn is_weak_field_nullable_type(&self, ty: TypeId) -> bool {
        let ty = self.resolve_alias_type(ty);
        let null_type = self.layer.table().primitive(PrimitiveType::Null);

        match self.layer.table().kind(ty) {
            Some(TypeKind::Union { members }) => members.contains(&null_type),
            Some(TypeKind::Primitive(PrimitiveType::Null)) => true,
            _ => false,
        }
    }

    fn report_invalid_weak_field_type(&mut self, field: NodeId, field_type: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(field)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let field_type = self.describe_type_for_diagnostic(field_type);

        self.diagnostics
            .push(galfus_core::Diagnostic::error_with_message(
                TypeDiagnosticCode::InvalidWeakFieldType,
                format!("weak field type must be nullable, got `{field_type}`"),
                span,
            ));
    }
}
