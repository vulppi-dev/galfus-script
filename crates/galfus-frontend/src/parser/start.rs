use super::*;

impl Parser {
    pub(super) fn source_file_span(&self) -> Span {
        let end = self
            .tokens
            .last()
            .map(|token| token.span().start())
            .unwrap_or(0);

        Span::new(self.graph.source_id(), 0, end)
    }

    pub(super) fn parse_source_file(&mut self) {
        let span = self.source_file_span();

        let root = self.add_node(SyntaxNodeKind::SourceFile, span, Vec::new());

        self.graph.syntax_mut().set_root(root);

        while !self.is_eof() {
            self.skip_newlines();

            if self.is_eof() {
                break;
            }

            let start_position = self.position;

            if let Some(item) = self.parse_item() {
                self.graph.syntax_mut().push_child(root, item);
            }

            if self.position == start_position {
                self.bump();
            }
        }
    }

    pub(super) fn parse_item(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Import) {
            return self.parse_import_item();
        }

        if self.at(&TokenKind::Fn) {
            return self.parse_function_item();
        }

        if self.at(&TokenKind::Type) {
            return self.parse_type_alias_item();
        }

        if self.at(&TokenKind::Struct) {
            return self.parse_struct_item();
        }

        if self.at(&TokenKind::Enum) {
            return self.parse_enum_item();
        }

        if self.at(&TokenKind::Choice) {
            return self.parse_choice_item();
        }

        if self.at(&TokenKind::Export) {
            return self.parse_export_item();
        }

        let found = self.bump();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::ExpectedItem,
            format!("expected item, found `{:?}`", found.kind()),
            found.span(),
        ));

        None
    }
}
