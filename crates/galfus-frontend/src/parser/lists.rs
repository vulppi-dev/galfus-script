use super::*;

impl Parser {
    pub(super) fn parse_parameter_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut parameters = Vec::new();

        let mut seen_rest_parameter = false;
        let mut reported_after_rest = false;

        self.skip_newlines();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            let start_position = self.position;

            let parameter = self.parse_parameter();

            if let Some(parameter) = parameter {
                let is_rest_parameter = self
                    .graph
                    .syntax()
                    .node(parameter)
                    .map(|node| node.kind() == SyntaxNodeKind::RestParameter)
                    .unwrap_or(false);

                if seen_rest_parameter && !reported_after_rest {
                    self.graph.push_diagnostic(Diagnostic::error_with_message(
                        ParserDiagnosticCode::UnexpectedToken,
                        "rest parameter must be the last parameter".to_string(),
                        self.node_span(parameter),
                    ));
                    reported_after_rest = true;
                }

                seen_rest_parameter = is_rest_parameter;
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

            if self.at(&TokenKind::DotDotDot) {
                let expansion = self.parse_struct_expansion()?;
                fields.push(expansion);
            } else {
                let field = self.parse_struct_field()?;
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

        self.skip_newlines();

        let mut arguments = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                let comma = self.bump();

                let omitted =
                    self.add_node(SyntaxNodeKind::OmittedArgument, comma.span(), Vec::new());

                arguments.push(omitted);

                self.skip_newlines();
                continue;
            }

            let argument = self.parse_argument()?;
            arguments.push(argument);

            self.skip_newlines();

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();

                if self.at(&TokenKind::RightParen) {
                    break;
                }

                continue;
            }

            break;
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

            if self.at(&TokenKind::DotDotDot) {
                let field = self.parse_spread_struct_literal_field()?;
                fields.push(field);
            } else {
                let field = self.parse_struct_literal_field()?;
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

    pub(super) fn parse_type_argument_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::Less)?;

        let mut arguments = Vec::new();

        self.skip_newlines();

        while !self.is_eof() && !self.at_type_argument_close() {
            let start_position = self.position;

            if let Some(argument) = self.parse_type() {
                arguments.push(argument);
            }

            self.skip_newlines();

            if self.at_type_argument_close() {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();

                self.skip_newlines();

                if self.at_type_argument_close() {
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

        let right = self.expect_type_argument_close()?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::TypeArgumentList, span, arguments))
    }

    pub(super) fn parse_generic_argument_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::Less)?;

        let mut arguments = Vec::new();

        self.skip_newlines();

        while !self.is_eof() && !self.at_type_argument_close() {
            let start_position = self.position;

            if let Some(argument) = self.parse_type() {
                arguments.push(argument);
            }

            self.skip_newlines();

            if self.at_type_argument_close() {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();

                self.skip_newlines();

                if self.at_type_argument_close() {
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

        let right = self.expect_type_argument_close()?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::GenericArgumentList, span, arguments))
    }

    pub(super) fn parse_generic_parameter_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::Less)?;

        let mut parameters = Vec::new();

        self.skip_newlines();

        while !self.is_eof() && !self.at_type_argument_close() {
            let start_position = self.position;

            if let Some(parameter) = self.parse_generic_parameter() {
                parameters.push(parameter);
            }

            self.skip_newlines();

            if self.at_type_argument_close() {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();

                self.skip_newlines();

                if self.at_type_argument_close() {
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

        let right = self.expect_type_argument_close()?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::GenericParameterList, span, parameters))
    }

    pub(super) fn parse_constraint_member_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut members = Vec::new();

        self.skip_newlines();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            if self.at(&TokenKind::Fn) {
                if let Some(signature) = self.parse_constraint_function_signature() {
                    members.push(signature);
                }

                self.skip_newlines();

                if self.at(&TokenKind::Comma) {
                    self.bump();
                    self.skip_newlines();
                    continue;
                }

                if !self.at(&TokenKind::RightBrace) {
                    let found = self.current().clone();

                    self.graph.push_diagnostic(Diagnostic::error_with_message(
                        ParserDiagnosticCode::ExpectedToken,
                        format!("expected `Comma`, found `{:?}`", found.kind()),
                        found.span(),
                    ));

                    self.bump();
                }

                continue;
            }

            if self.at(&TokenKind::Identifier) {
                if let Some(field) = self.parse_constraint_field() {
                    members.push(field);
                }

                self.skip_newlines();

                if self.at(&TokenKind::Comma) {
                    self.bump();
                    self.skip_newlines();
                    continue;
                }

                if !self.at(&TokenKind::RightBrace) {
                    let found = self.current().clone();

                    self.graph.push_diagnostic(Diagnostic::error_with_message(
                        ParserDiagnosticCode::ExpectedToken,
                        format!("expected `Comma`, found `{:?}`", found.kind()),
                        found.span(),
                    ));

                    self.bump();
                }

                continue;
            }

            let found = self.bump();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedItem,
                format!("expected constraint member, found `{:?}`", found.kind()),
                found.span(),
            ));

            self.skip_newlines();
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ConstraintMemberList, span, members))
    }

    pub(super) fn parse_function_type_parameter_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut parameters = Vec::new();

        self.skip_newlines();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            let start_position = self.position;

            if let Some(parameter_type) = self.parse_type() {
                parameters.push(parameter_type);
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

        Some(self.add_node(SyntaxNodeKind::FunctionTypeParameterList, span, parameters))
    }
}
