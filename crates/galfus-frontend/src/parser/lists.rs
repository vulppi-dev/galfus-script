use super::*;

impl Parser {
    pub(super) fn parse_parameter_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut parameters = Vec::new();

        let mut seen_rest_parameter = false;
        let mut seen_default_parameter = false;

        let mut reported_after_rest = false;
        let mut reported_required_after_default = false;
        let mut reported_rest_without_default_after_default = false;

        self.skip_newlines();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            let start_position = self.position;

            let starts_after_rest = seen_rest_parameter;
            let is_rest_parameter = self.at(&TokenKind::DotDotDot);

            let parameter = if is_rest_parameter {
                self.parse_rest_parameter()
            } else {
                self.parse_parameter()
            };

            if let Some(parameter) = parameter {
                let has_default = self.parameter_has_default(parameter);

                if starts_after_rest && !reported_after_rest {
                    self.graph.push_diagnostic(Diagnostic::error_with_message(
                        ParserDiagnosticCode::UnexpectedToken,
                        "rest parameter must be the last parameter",
                        self.node_span(parameter),
                    ));

                    reported_after_rest = true;
                }

                if is_rest_parameter {
                    if seen_default_parameter
                        && !has_default
                        && !reported_rest_without_default_after_default
                    {
                        self.graph.push_diagnostic(Diagnostic::error_with_message(
                            ParserDiagnosticCode::UnexpectedToken,
                            "rest parameter after default parameter must also have default",
                            self.node_span(parameter),
                        ));

                        reported_rest_without_default_after_default = true;
                    }

                    seen_rest_parameter = true;
                } else {
                    if seen_default_parameter && !has_default && !reported_required_after_default {
                        self.graph.push_diagnostic(Diagnostic::error_with_message(
                            ParserDiagnosticCode::UnexpectedToken,
                            "required parameter cannot follow default parameter",
                            self.node_span(parameter),
                        ));

                        reported_required_after_default = true;
                    }
                }

                if has_default {
                    seen_default_parameter = true;
                }

                parameters.push(parameter);
            }

            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();

                self.skip_newlines();

                if self.at(&TokenKind::RightParen) {
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

    pub(super) fn parse_match_arm_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut arms = Vec::new();

        self.skip_newlines();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            let start_position = self.position;

            if let Some(arm) = self.parse_match_arm() {
                arms.push(arm);
            }

            self.skip_newlines();

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::MatchArmList, span, arms))
    }

    pub(super) fn parse_instanceof_arm_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut arms = Vec::new();

        self.skip_newlines();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            let start_position = self.position;

            if let Some(arm) = self.parse_instanceof_arm() {
                arms.push(arm);
            }

            self.skip_newlines();

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::InstanceofArmList, span, arms))
    }
}
