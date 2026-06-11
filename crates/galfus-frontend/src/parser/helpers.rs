use super::*;

impl Parser {
    pub(super) fn can_start_expression(&self) -> bool {
        matches!(
            self.current().kind(),
            TokenKind::Identifier
                | TokenKind::Integer
                | TokenKind::Float
                | TokenKind::String
                | TokenKind::Regex
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::LeftParen
                | TokenKind::LeftBracket
                | TokenKind::Struct
                | TokenKind::Copy
                | TokenKind::Minus
                | TokenKind::Bang
                | TokenKind::Tilde
        )
    }

    pub(super) fn can_continue_expression_after_newline(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Dot
                | TokenKind::ColonColon
                | TokenKind::LeftBracket
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::StarStar
                | TokenKind::Less
                | TokenKind::LessEqual
                | TokenKind::Greater
                | TokenKind::GreaterEqual
                | TokenKind::EqualEqual
                | TokenKind::BangEqual
                | TokenKind::AmpAmp
                | TokenKind::PipePipe
                | TokenKind::QuestionQuestion
        )
    }

    pub(super) fn binary_operator_info(kind: &TokenKind) -> Option<(u8, BinaryAssociativity)> {
        match kind {
            TokenKind::StarStar => Some((80, BinaryAssociativity::Right)),

            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => {
                Some((70, BinaryAssociativity::Left))
            }

            TokenKind::Plus | TokenKind::Minus => Some((60, BinaryAssociativity::Left)),

            TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => Some((50, BinaryAssociativity::Left)),

            TokenKind::EqualEqual | TokenKind::BangEqual => Some((45, BinaryAssociativity::Left)),

            TokenKind::AmpAmp => Some((30, BinaryAssociativity::Left)),

            TokenKind::PipePipe => Some((20, BinaryAssociativity::Left)),

            TokenKind::QuestionQuestion => Some((10, BinaryAssociativity::Right)),

            _ => None,
        }
    }

    pub(super) fn is_unary_operator(kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::Minus | TokenKind::Bang | TokenKind::Tilde)
    }

    pub(super) fn is_assignment_operator(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Equal
                | TokenKind::PlusEqual
                | TokenKind::MinusEqual
                | TokenKind::StarEqual
                | TokenKind::SlashEqual
                | TokenKind::PercentEqual
                | TokenKind::StarStarEqual
                | TokenKind::AmpEqual
                | TokenKind::PipeEqual
                | TokenKind::CaretEqual
                | TokenKind::ShiftLeftEqual
                | TokenKind::ShiftRightEqual
        )
    }

    pub(super) fn expression_can_be_assignment_target(&self, expression: NodeId) -> bool {
        let Some(node) = self.graph.syntax().node(expression) else {
            return false;
        };

        matches!(
            node.kind(),
            SyntaxNodeKind::NameExpression
                | SyntaxNodeKind::MemberExpression
                | SyntaxNodeKind::IndexExpression
        )
    }

    pub(super) fn expression_can_be_statement(&self, expression: NodeId) -> bool {
        let Some(node) = self.graph.syntax().node(expression) else {
            return false;
        };

        match node.kind() {
            SyntaxNodeKind::CallExpression => true,

            SyntaxNodeKind::GroupedExpression => node
                .children()
                .first()
                .copied()
                .map(|child| self.expression_can_be_statement(child))
                .unwrap_or(false),

            _ => false,
        }
    }

    pub(super) fn expect_statement_end(&mut self) {
        if self.at(&TokenKind::Newline) {
            self.skip_newlines();
            return;
        }

        if self.at(&TokenKind::Semicolon) {
            self.bump();
            self.skip_newlines();
            return;
        }

        if self.at(&TokenKind::RightBrace) || self.is_eof() {
            return;
        }

        let found = self.current().clone();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::ExpectedToken,
            format!("expected statement terminator, found `{:?}`", found.kind()),
            found.span(),
        ));
    }

    pub(super) fn parameter_has_default(&self, parameter: NodeId) -> bool {
        let Some(parameter_node) = self.graph.syntax().node(parameter) else {
            return false;
        };

        parameter_node
            .children()
            .iter()
            .any(|child| match self.graph.syntax().node(*child) {
                Some(node) => node.kind() == SyntaxNodeKind::ParameterDefault,
                None => false,
            })
    }

    pub(super) fn is_arrow_function_start(&self) -> bool {
        if !self.at(&TokenKind::LeftParen) {
            return false;
        }

        let mut index = self.position;
        let mut depth = 0usize;

        while index < self.tokens.len() {
            match self.tokens[index].kind() {
                TokenKind::LeftParen => {
                    depth += 1;
                }
                TokenKind::RightParen => {
                    depth -= 1;

                    if depth == 0 {
                        index += 1;

                        while index < self.tokens.len()
                            && self.tokens[index].kind() == &TokenKind::Newline
                        {
                            index += 1;
                        }

                        return matches!(
                            self.tokens.get(index).map(|token| token.kind()),
                            Some(TokenKind::Arrow) | Some(TokenKind::Colon)
                        );
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }

            index += 1;
        }

        false
    }

    pub(super) fn at_type_argument_close(&self) -> bool {
        matches!(
            self.current().kind(),
            TokenKind::Greater
                | TokenKind::ShiftRight
                | TokenKind::GreaterEqual
                | TokenKind::ShiftRightEqual
        )
    }

    pub(super) fn split_current_type_argument_close(&mut self) {
        let token = self.current().clone();
        let span = token.span();

        let source_id = span.source_id();
        let start = span.start();
        let end = span.end();

        match token.kind() {
            TokenKind::ShiftRight => {
                self.tokens[self.position] =
                    Token::new(TokenKind::Greater, Span::new(source_id, start, start + 1));

                self.tokens.insert(
                    self.position + 1,
                    Token::new(TokenKind::Greater, Span::new(source_id, start + 1, end)),
                );
            }

            TokenKind::GreaterEqual => {
                self.tokens[self.position] =
                    Token::new(TokenKind::Greater, Span::new(source_id, start, start + 1));

                self.tokens.insert(
                    self.position + 1,
                    Token::new(TokenKind::Equal, Span::new(source_id, start + 1, end)),
                );
            }

            TokenKind::ShiftRightEqual => {
                self.tokens[self.position] =
                    Token::new(TokenKind::Greater, Span::new(source_id, start, start + 1));

                self.tokens.insert(
                    self.position + 1,
                    Token::new(
                        TokenKind::Greater,
                        Span::new(source_id, start + 1, start + 2),
                    ),
                );

                self.tokens.insert(
                    self.position + 2,
                    Token::new(TokenKind::Equal, Span::new(source_id, start + 2, end)),
                );
            }

            _ => {}
        }
    }

    pub(super) fn expect_type_argument_close(&mut self) -> Option<Token> {
        if matches!(
            self.current().kind(),
            TokenKind::ShiftRight | TokenKind::GreaterEqual | TokenKind::ShiftRightEqual
        ) {
            self.split_current_type_argument_close();
        }

        self.expect(TokenKind::Greater)
    }

    pub(super) fn can_parse_generic_call_suffix(&self) -> bool {
        if !self.at(&TokenKind::Less) {
            return false;
        }

        let mut index = self.position;
        let mut depth = 0usize;

        while index < self.tokens.len() {
            match self.tokens[index].kind() {
                TokenKind::Less => {
                    depth += 1;
                }

                TokenKind::Greater => {
                    depth -= 1;

                    if depth == 0 {
                        index += 1;

                        while index < self.tokens.len()
                            && self.tokens[index].kind() == &TokenKind::Newline
                        {
                            index += 1;
                        }

                        return self
                            .tokens
                            .get(index)
                            .is_some_and(|token| token.kind() == &TokenKind::LeftParen);
                    }
                }

                TokenKind::ShiftRight => {
                    if depth >= 2 {
                        depth -= 2;

                        if depth == 0 {
                            index += 1;

                            while index < self.tokens.len()
                                && self.tokens[index].kind() == &TokenKind::Newline
                            {
                                index += 1;
                            }

                            return self
                                .tokens
                                .get(index)
                                .is_some_and(|token| token.kind() == &TokenKind::LeftParen);
                        }
                    } else {
                        return false;
                    }
                }

                TokenKind::Eof => return false,

                _ => {}
            }

            index += 1;
        }

        false
    }

    pub(super) fn find_function_anchor_separator(&self) -> Option<usize> {
        let mut index = self.position;
        let mut depth = 0usize;
        let mut separator = None;

        while index < self.tokens.len() {
            match self.tokens[index].kind() {
                TokenKind::Less => {
                    depth += 1;
                }

                TokenKind::Greater => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }

                TokenKind::ShiftRight => {
                    if depth >= 2 {
                        depth -= 2;
                    } else if depth == 1 {
                        depth = 0;
                    }
                }

                TokenKind::ColonColon if depth == 0 => {
                    let next = index + 1;

                    if matches!(
                        self.tokens.get(next).map(|token| token.kind()),
                        Some(TokenKind::Identifier)
                    ) {
                        separator = Some(index);
                    }
                }

                TokenKind::LeftParen if depth == 0 => {
                    return separator;
                }

                TokenKind::Eof | TokenKind::Newline if depth == 0 => {
                    return separator;
                }

                _ => {}
            }

            index += 1;
        }

        separator
    }

    pub(super) fn parse_binding_after_keyword(
        &mut self,
        require_initializer: bool,
    ) -> Option<(Vec<NodeId>, Span)> {
        self.skip_newlines();

        let name = self.parse_identifier()?;
        let mut children = vec![name];
        let mut end_span = self.node_span(name);

        self.skip_newlines();

        if self.at(&TokenKind::Colon) {
            let annotation = self.parse_type_annotation()?;
            end_span = self.node_span(annotation);
            children.push(annotation);
        }

        self.skip_newlines();

        if self.at(&TokenKind::Equal) {
            let initializer = self.parse_initializer()?;
            end_span = self.node_span(initializer);
            children.push(initializer);
        } else if require_initializer {
            let token = self.current();

            self.graph.push_diagnostic(Diagnostic::error(
                ParserDiagnosticCode::ExpectedInitializer,
                token.span(),
            ));

            return None;
        }

        Some((children, end_span))
    }
}
