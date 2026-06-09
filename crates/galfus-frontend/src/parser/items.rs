use super::*;

impl Parser {
    pub(super) fn parse_export_item(&mut self) -> Option<NodeId> {
        let export_token = self.expect(TokenKind::Export)?;

        let item = if self.at(&TokenKind::Fn) {
            self.parse_function_item()?
        } else if self.at(&TokenKind::Type) {
            self.parse_type_alias_item()?
        } else if self.at(&TokenKind::Struct) {
            self.parse_struct_item()?
        } else if self.at(&TokenKind::Enum) {
            self.parse_enum_item()?
        } else if self.at(&TokenKind::Choice) {
            self.parse_choice_item()?
        } else {
            let found = self.bump();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedItem,
                format!("expected exportable item, found `{:?}`", found.kind()),
                found.span(),
            ));

            return None;
        };

        let span =
            Span::cover(export_token.span(), self.node_span(item)).unwrap_or(export_token.span());

        Some(self.add_node(SyntaxNodeKind::ExportItem, span, vec![item]))
    }

    pub(super) fn parse_import_item(&mut self) -> Option<NodeId> {
        let import_token = self.expect(TokenKind::Import)?;

        let clause = self.parse_import_clause()?;

        self.expect(TokenKind::From)?;

        let source = self.parse_import_source()?;

        let span =
            Span::cover(import_token.span(), self.node_span(source)).unwrap_or(import_token.span());

        Some(self.add_node(SyntaxNodeKind::ImportItem, span, vec![clause, source]))
    }

    pub(super) fn parse_function_item(&mut self) -> Option<NodeId> {
        let fn_token = self.expect(TokenKind::Fn)?;

        let name = self.parse_identifier()?;
        let parameters = self.parse_parameter_list()?;

        self.expect(TokenKind::Colon)?;

        let return_type = self.parse_type()?;
        let body = self.parse_block()?;

        let span = Span::cover(fn_token.span(), self.node_span(body)).unwrap_or(fn_token.span());

        Some(self.add_node(
            SyntaxNodeKind::FunctionItem,
            span,
            vec![name, parameters, return_type, body],
        ))
    }

    pub(super) fn parse_type_alias_item(&mut self) -> Option<NodeId> {
        let type_token = self.expect(TokenKind::Type)?;

        let name = self.parse_identifier()?;

        self.expect(TokenKind::Equal)?;

        let aliased_type = self.parse_type()?;

        let span = Span::cover(type_token.span(), self.node_span(aliased_type))
            .unwrap_or(type_token.span());

        Some(self.add_node(
            SyntaxNodeKind::TypeAliasItem,
            span,
            vec![name, aliased_type],
        ))
    }

    pub(super) fn parse_struct_item(&mut self) -> Option<NodeId> {
        let struct_token = self.expect(TokenKind::Struct)?;

        let name = self.parse_identifier()?;
        let fields = self.parse_struct_field_list()?;

        let span =
            Span::cover(struct_token.span(), self.node_span(fields)).unwrap_or(struct_token.span());

        Some(self.add_node(SyntaxNodeKind::StructItem, span, vec![name, fields]))
    }

    pub(super) fn parse_enum_item(&mut self) -> Option<NodeId> {
        let enum_token = self.expect(TokenKind::Enum)?;

        let name = self.parse_identifier()?;
        let variants = self.parse_enum_variant_list()?;

        let span =
            Span::cover(enum_token.span(), self.node_span(variants)).unwrap_or(enum_token.span());

        Some(self.add_node(SyntaxNodeKind::EnumItem, span, vec![name, variants]))
    }

    pub(super) fn parse_choice_item(&mut self) -> Option<NodeId> {
        let choice_token = self.expect(TokenKind::Choice)?;

        let name = self.parse_identifier()?;
        let variants = self.parse_choice_variant_list()?;

        let span = Span::cover(choice_token.span(), self.node_span(variants))
            .unwrap_or(choice_token.span());

        Some(self.add_node(SyntaxNodeKind::ChoiceItem, span, vec![name, variants]))
    }
}
