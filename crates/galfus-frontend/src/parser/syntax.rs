use super::*;

impl Parser {
    pub(super) fn parse_identifier(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Identifier)?;

        Some(self.add_node(SyntaxNodeKind::Identifier, token.span(), Vec::new()))
    }

    pub(super) fn parse_type(&mut self) -> Option<NodeId> {
        let first = self.parse_primary_type()?;

        if !self.at(&TokenKind::Pipe) {
            return Some(first);
        }

        let mut types = vec![first];
        let start_span = self.node_span(first);

        while self.at(&TokenKind::Pipe) {
            self.bump();

            let next = self.parse_primary_type()?;
            types.push(next);
        }

        let last = *types
            .last()
            .expect("union type must have at least one type");
        let span = Span::cover(start_span, self.node_span(last)).unwrap_or(start_span);

        Some(self.add_node(SyntaxNodeKind::UnionType, span, types))
    }

    pub(super) fn parse_primary_type(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Null) {
            let token = self.bump();

            return Some(self.add_node(SyntaxNodeKind::TypeNull, token.span(), Vec::new()));
        }

        if self.at(&TokenKind::LeftBracket) {
            return self.parse_array_type();
        }

        if self.at(&TokenKind::Identifier) {
            let identifier = self.parse_identifier()?;
            let span = self.node_span(identifier);

            return Some(self.add_node(SyntaxNodeKind::TypeName, span, vec![identifier]));
        }

        let found = self.bump();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::ExpectedType,
            format!("expected type, found `{:?}`", found.kind()),
            found.span(),
        ));

        None
    }

    pub(super) fn parse_block(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut statements = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if let Some(statement) = self.parse_statement() {
                statements.push(statement);
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::Block, span, statements))
    }

    pub(super) fn parse_parameter(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;
        let name_span = self.node_span(name);

        self.skip_newlines();

        self.expect(TokenKind::Colon)?;

        self.skip_newlines();

        let parameter_type = self.parse_type()?;

        let mut children = vec![name, parameter_type];
        let mut end_span = self.node_span(parameter_type);

        self.skip_newlines();

        if self.at(&TokenKind::Equal) {
            let default = self.parse_parameter_default()?;
            end_span = self.node_span(default);
            children.push(default);
        }

        let span = Span::cover(name_span, end_span).unwrap_or(name_span);

        Some(self.add_node(SyntaxNodeKind::Parameter, span, children))
    }

    pub(super) fn parse_rest_parameter(&mut self) -> Option<NodeId> {
        let spread_token = self.expect(TokenKind::DotDotDot)?;

        self.skip_newlines();

        let name = self.parse_identifier()?;

        self.skip_newlines();

        self.expect(TokenKind::Colon)?;

        self.skip_newlines();

        let parameter_type = self.parse_type()?;

        let mut children = vec![name, parameter_type];
        let mut end_span = self.node_span(parameter_type);

        self.skip_newlines();

        if self.at(&TokenKind::Equal) {
            let default = self.parse_parameter_default()?;
            end_span = self.node_span(default);
            children.push(default);
        }

        let span = Span::cover(spread_token.span(), end_span).unwrap_or(spread_token.span());

        Some(self.add_node(SyntaxNodeKind::RestParameter, span, children))
    }

    pub(super) fn parse_parameter_default(&mut self) -> Option<NodeId> {
        let equal = self.expect(TokenKind::Equal)?;

        self.skip_newlines();

        let value = self.parse_expression()?;

        let span = Span::cover(equal.span(), self.node_span(value)).unwrap_or(equal.span());

        Some(self.add_node(SyntaxNodeKind::ParameterDefault, span, vec![value]))
    }

    pub(super) fn parse_array_type(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBracket)?;

        let element_type = self.parse_type()?;

        if self.at(&TokenKind::Semicolon) {
            self.bump();

            let size = self.parse_array_size()?;
            let right = self.expect(TokenKind::RightBracket)?;

            let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

            return Some(self.add_node(
                SyntaxNodeKind::FixedArrayType,
                span,
                vec![element_type, size],
            ));
        }

        let right = self.expect(TokenKind::RightBracket)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ArrayType, span, vec![element_type]))
    }

    pub(super) fn parse_array_size(&mut self) -> Option<NodeId> {
        if !self.at(&TokenKind::Integer) {
            let found = self.bump();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!(
                    "expected array size integer literal, found `{:?}`",
                    found.kind()
                ),
                found.span(),
            ));

            return None;
        }

        let value = self.parse_integer_literal()?;
        let span = self.node_span(value);

        Some(self.add_node(SyntaxNodeKind::ArraySize, span, vec![value]))
    }

    pub(super) fn parse_import_clause(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::LeftBrace) {
            return self.parse_named_import_list();
        }

        self.parse_namespace_import()
    }

    pub(super) fn parse_namespace_import(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;
        let span = self.node_span(name);

        Some(self.add_node(SyntaxNodeKind::NamespaceImport, span, vec![name]))
    }

    pub(super) fn parse_named_import(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;

        let mut children = vec![name];
        let mut end_span = self.node_span(name);

        if self.at(&TokenKind::As) {
            let alias = self.parse_import_alias()?;
            end_span = self.node_span(alias);
            children.push(alias);
        }

        let span =
            Span::cover(self.node_span(name), end_span).unwrap_or_else(|| self.node_span(name));

        Some(self.add_node(SyntaxNodeKind::NamedImport, span, children))
    }

    pub(super) fn parse_import_source(&mut self) -> Option<NodeId> {
        let literal = self.parse_string_literal()?;
        let span = self.node_span(literal);

        Some(self.add_node(SyntaxNodeKind::ImportSource, span, vec![literal]))
    }

    pub(super) fn parse_import_alias(&mut self) -> Option<NodeId> {
        let as_token = self.expect(TokenKind::As)?;
        let name = self.parse_identifier()?;

        let span = Span::cover(as_token.span(), self.node_span(name)).unwrap_or(as_token.span());

        Some(self.add_node(SyntaxNodeKind::ImportAlias, span, vec![name]))
    }

    pub(super) fn parse_struct_field(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;

        self.expect(TokenKind::Colon)?;

        let field_type = self.parse_type()?;

        let span = Span::cover(self.node_span(name), self.node_span(field_type))
            .unwrap_or_else(|| self.node_span(name));

        Some(self.add_node(SyntaxNodeKind::StructField, span, vec![name, field_type]))
    }

    pub(super) fn parse_enum_variant(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;
        let span = self.node_span(name);

        Some(self.add_node(SyntaxNodeKind::EnumVariant, span, vec![name]))
    }

    pub(super) fn parse_choice_payload(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut payload_types = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            let start_position = self.position;

            if let Some(payload_type) = self.parse_type() {
                payload_types.push(payload_type);
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

        Some(self.add_node(SyntaxNodeKind::ChoicePayload, span, payload_types))
    }

    pub(super) fn parse_choice_variant(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;

        let mut children = vec![name];
        let mut end_span = self.node_span(name);

        if self.at(&TokenKind::LeftParen) {
            let payload = self.parse_choice_payload()?;
            end_span = self.node_span(payload);
            children.push(payload);
        }

        let span =
            Span::cover(self.node_span(name), end_span).unwrap_or_else(|| self.node_span(name));

        Some(self.add_node(SyntaxNodeKind::ChoiceVariant, span, children))
    }

    pub(super) fn parse_type_annotation(&mut self) -> Option<NodeId> {
        let colon = self.expect(TokenKind::Colon)?;
        let type_node = self.parse_type()?;

        let span = Span::cover(colon.span(), self.node_span(type_node)).unwrap_or(colon.span());

        Some(self.add_node(SyntaxNodeKind::TypeAnnotation, span, vec![type_node]))
    }

    pub(super) fn parse_initializer(&mut self) -> Option<NodeId> {
        let equal = self.expect(TokenKind::Equal)?;
        let expression = self.parse_expression()?;

        let span = Span::cover(equal.span(), self.node_span(expression)).unwrap_or(equal.span());

        Some(self.add_node(SyntaxNodeKind::Initializer, span, vec![expression]))
    }

    pub(super) fn parse_argument(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::DotDotDot) {
            return self.parse_spread_argument();
        }

        let expression = self.parse_expression()?;
        let span = self.node_span(expression);

        Some(self.add_node(SyntaxNodeKind::Argument, span, vec![expression]))
    }
}
