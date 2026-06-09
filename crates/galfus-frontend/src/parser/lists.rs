use super::*;

impl Parser {
    pub(super) fn parse_parameter_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut parameters = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            let start_position = self.position;

            if let Some(parameter) = self.parse_parameter() {
                parameters.push(parameter);
            }

            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ParameterList, span, parameters))
    }

    pub(super) fn parse_named_import_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut imports = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if let Some(import) = self.parse_named_import() {
                imports.push(import);
            }

            if !self.at(&TokenKind::Comma) {
                break;
            }

            self.bump();
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::NamedImportList, span, imports))
    }

    pub(super) fn parse_struct_field_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut fields = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            let start_position = self.position;

            if let Some(field) = self.parse_struct_field() {
                fields.push(field);
            }

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();

                if self.at(&TokenKind::RightBrace) {
                    break;
                }

                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::StructFieldList, span, fields))
    }

    pub(super) fn parse_enum_variant_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut variants = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            let start_position = self.position;

            if let Some(variant) = self.parse_enum_variant() {
                variants.push(variant);
            }

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();

                if self.at(&TokenKind::RightBrace) {
                    break;
                }

                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::EnumVariantList, span, variants))
    }

    pub(super) fn parse_choice_variant_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut variants = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            let start_position = self.position;

            if let Some(variant) = self.parse_choice_variant() {
                variants.push(variant);
            }

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();

                if self.at(&TokenKind::RightBrace) {
                    break;
                }

                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ChoiceVariantList, span, variants))
    }

    pub(super) fn parse_argument_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut arguments = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            let start_position = self.position;

            if let Some(argument) = self.parse_argument() {
                arguments.push(argument);
            }

            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ArgumentList, span, arguments))
    }

    pub(super) fn parse_struct_literal_field_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut fields = Vec::new();

        self.skip_newlines();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            let start_position = self.position;

            if let Some(field) = self.parse_struct_literal_field() {
                fields.push(field);
            }

            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();

                if self.at(&TokenKind::RightBrace) {
                    break;
                }

                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::StructLiteralFieldList, span, fields))
    }
}
