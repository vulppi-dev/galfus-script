use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExpressionBoundary {
    None,
    BeforeBlock,
}

impl Parser {
    pub(super) fn parse_expression(&mut self) -> Option<NodeId> {
        self.parse_expression_with_boundary(ExpressionBoundary::None)
    }

    pub(super) fn parse_expression_before_block(&mut self) -> Option<NodeId> {
        self.parse_expression_with_boundary(ExpressionBoundary::BeforeBlock)
    }

    fn parse_expression_with_boundary(&mut self, boundary: ExpressionBoundary) -> Option<NodeId> {
        self.parse_binary_expression(0, boundary)
    }

    pub(super) fn parse_name_expression(&mut self) -> Option<NodeId> {
        let identifier = self.parse_identifier()?;
        let span = self.node_span(identifier);

        Some(self.add_node(SyntaxNodeKind::NameExpression, span, vec![identifier]))
    }

    pub(super) fn parse_primary_expression(
        &mut self,
        boundary: ExpressionBoundary,
    ) -> Option<NodeId> {
        if self.at(&TokenKind::LeftParen) && self.is_arrow_function_start() {
            return self.parse_arrow_function_expression();
        }

        if self.at(&TokenKind::LeftParen) {
            return self.parse_grouped_expression();
        }

        if self.at(&TokenKind::LeftBracket) {
            return self.parse_array_literal();
        }

        if self.at(&TokenKind::Struct) {
            return self.parse_inferred_struct_literal();
        }

        if self.at(&TokenKind::Integer) {
            return self.parse_integer_literal();
        }

        if self.at(&TokenKind::Float) {
            return self.parse_float_literal();
        }

        if self.at(&TokenKind::String) {
            return self.parse_string_literal();
        }

        if self.at(&TokenKind::True) || self.at(&TokenKind::False) {
            return self.parse_bool_literal();
        }

        if self.at(&TokenKind::Null) {
            return self.parse_null_literal();
        }

        if self.at(&TokenKind::Identifier) {
            let next = self.peek_after_newlines(1).kind();

            if next == &TokenKind::LeftBrace && boundary != ExpressionBoundary::BeforeBlock {
                return self.parse_struct_literal();
            }

            return self.parse_name_expression();
        }

        let found = self.bump();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::UnexpectedToken,
            format!("expected expression, found `{:?}`", found.kind()),
            found.span(),
        ));

        None
    }

    fn parse_postfix_expression(&mut self, boundary: ExpressionBoundary) -> Option<NodeId> {
        let mut expression = self.parse_primary_expression(boundary)?;

        loop {
            self.skip_soft_newlines_before_expression_continuation();

            if self.at(&TokenKind::Dot) {
                expression = self.parse_member_expression(expression)?;
                continue;
            }

            if self.at(&TokenKind::ColonColon) {
                expression = self.parse_anchor_expression(expression)?;
                continue;
            }

            if self.at(&TokenKind::LeftBracket) {
                expression = self.parse_index_expression(expression)?;
                continue;
            }

            if self.at(&TokenKind::LeftParen) {
                expression = self.parse_call_expression(expression)?;
                continue;
            }

            break;
        }

        Some(expression)
    }

    pub(super) fn parse_call_expression(&mut self, target: NodeId) -> Option<NodeId> {
        let arguments = self.parse_argument_list()?;

        let span = Span::cover(self.node_span(target), self.node_span(arguments))
            .unwrap_or_else(|| self.node_span(target));

        Some(self.add_node(
            SyntaxNodeKind::CallExpression,
            span,
            vec![target, arguments],
        ))
    }

    pub(super) fn parse_member_expression(&mut self, target: NodeId) -> Option<NodeId> {
        self.expect(TokenKind::Dot)?;

        self.skip_newlines();

        let member = self.parse_identifier()?;

        let span = Span::cover(self.node_span(target), self.node_span(member))
            .unwrap_or_else(|| self.node_span(target));

        Some(self.add_node(SyntaxNodeKind::MemberExpression, span, vec![target, member]))
    }

    pub(super) fn parse_anchor_expression(&mut self, target: NodeId) -> Option<NodeId> {
        self.expect(TokenKind::ColonColon)?;

        self.skip_newlines();

        let anchor = self.parse_identifier()?;

        let span = Span::cover(self.node_span(target), self.node_span(anchor))
            .unwrap_or_else(|| self.node_span(target));

        Some(self.add_node(SyntaxNodeKind::AnchorExpression, span, vec![target, anchor]))
    }

    fn parse_binary_expression(
        &mut self,
        min_precedence: u8,
        boundary: ExpressionBoundary,
    ) -> Option<NodeId> {
        let mut left = self.parse_unary_expression(boundary)?;

        loop {
            self.skip_soft_newlines_before_expression_continuation();

            let operator_token = self.current().clone();

            let Some((precedence, associativity)) =
                Self::binary_operator_info(operator_token.kind())
            else {
                break;
            };

            if precedence < min_precedence {
                break;
            }

            self.bump();

            let operator = self.add_node(
                SyntaxNodeKind::BinaryOperator,
                operator_token.span(),
                Vec::new(),
            );

            self.skip_newlines();

            let next_min_precedence = match associativity {
                BinaryAssociativity::Left => precedence + 1,
                BinaryAssociativity::Right => precedence,
            };

            let right = self.parse_binary_expression(next_min_precedence, boundary)?;

            let span = Span::cover(self.node_span(left), self.node_span(right))
                .unwrap_or_else(|| self.node_span(left));

            left = self.add_node(
                SyntaxNodeKind::BinaryExpression,
                span,
                vec![left, operator, right],
            );
        }

        Some(left)
    }

    pub(super) fn parse_grouped_expression(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        self.skip_newlines();

        if self.at(&TokenKind::RightParen) {
            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::UnexpectedToken,
                "expected expression, found `RightParen`",
                found.span(),
            ));

            return None;
        }

        let expression = self.parse_expression()?;

        self.skip_newlines();

        let right = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::GroupedExpression, span, vec![expression]))
    }

    fn parse_unary_expression(&mut self, boundary: ExpressionBoundary) -> Option<NodeId> {
        if Self::is_unary_operator(self.current().kind()) {
            let operator_token = self.bump();

            let operator = self.add_node(
                SyntaxNodeKind::UnaryOperator,
                operator_token.span(),
                Vec::new(),
            );

            self.skip_newlines();

            let operand = self.parse_unary_expression(boundary)?;

            let span = Span::cover(operator_token.span(), self.node_span(operand))
                .unwrap_or(operator_token.span());

            return Some(self.add_node(
                SyntaxNodeKind::UnaryExpression,
                span,
                vec![operator, operand],
            ));
        }

        self.parse_postfix_expression(boundary)
    }

    pub(super) fn parse_index_expression(&mut self, target: NodeId) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBracket)?;

        self.skip_newlines();

        if self.at(&TokenKind::RightBracket) {
            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::UnexpectedToken,
                "expected expression, found `RightBracket`",
                found.span(),
            ));

            return None;
        }

        let index = self.parse_expression()?;

        self.skip_newlines();

        let right = self.expect(TokenKind::RightBracket)?;
        let target_span = self.node_span(target);

        let span = Span::cover(target_span, right.span())
            .or_else(|| Span::cover(target_span, left.span()))
            .unwrap_or_else(|| self.node_span(target));

        Some(self.add_node(SyntaxNodeKind::IndexExpression, span, vec![target, index]))
    }

    pub(super) fn parse_spread_argument(&mut self) -> Option<NodeId> {
        let spread_token = self.expect(TokenKind::DotDotDot)?;

        self.skip_newlines();

        let expression = self.parse_expression()?;

        let span = Span::cover(spread_token.span(), self.node_span(expression))
            .unwrap_or(spread_token.span());

        Some(self.add_node(SyntaxNodeKind::SpreadArgument, span, vec![expression]))
    }

    pub(super) fn parse_spread_array_element(&mut self) -> Option<NodeId> {
        let spread_token = self.expect(TokenKind::DotDotDot)?;

        self.skip_newlines();

        let expression = self.parse_expression()?;

        let span = Span::cover(spread_token.span(), self.node_span(expression))
            .unwrap_or(spread_token.span());

        Some(self.add_node(SyntaxNodeKind::SpreadArrayElement, span, vec![expression]))
    }

    pub(super) fn parse_inferred_struct_literal(&mut self) -> Option<NodeId> {
        let struct_token = self.expect(TokenKind::Struct)?;

        self.skip_newlines();

        let fields = self.parse_struct_literal_field_list()?;

        let span =
            Span::cover(struct_token.span(), self.node_span(fields)).unwrap_or(struct_token.span());

        Some(self.add_node(SyntaxNodeKind::InferredStructLiteral, span, vec![fields]))
    }

    pub(super) fn parse_arrow_function_expression(&mut self) -> Option<NodeId> {
        let parameters = self.parse_parameter_list()?;

        self.skip_newlines();

        let return_type = if self.at(&TokenKind::Colon) {
            self.bump();

            self.skip_newlines();

            Some(self.parse_type()?)
        } else {
            None
        };

        self.skip_newlines();

        self.expect(TokenKind::Arrow)?;

        self.skip_newlines();

        let body = if self.at(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            self.parse_expression()?
        };

        let mut children = vec![parameters];

        if let Some(return_type) = return_type {
            children.push(return_type);
        }

        children.push(body);

        let span = Span::cover(self.node_span(parameters), self.node_span(body))
            .unwrap_or_else(|| self.node_span(parameters));

        Some(self.add_node(SyntaxNodeKind::ArrowFunctionExpression, span, children))
    }
}
